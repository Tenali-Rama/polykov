//! The hand-written rules parser: the spec §5 baseline. Syntax: ADR 0009 and `parse/API.md`.

mod marks;
mod recur;
mod when;

use chrono::{NaiveDate, NaiveTime};

use crate::words::{self, Run};
use crate::{Context, Kind, Parsed, Parser, Token};

/// Matches words left to right; at each word the first matcher that fits wins and consumes
/// its words. For a field given twice, the first value wins; every match is still a token.
#[derive(Debug, Clone, Copy, Default)]
pub struct RulesParser;

/// What one token contributes.
pub(crate) enum Hit {
    Date(NaiveDate),
    Time(NaiveTime),
    Duration(u32),
    Priority(u8),
    Project(Option<String>),
    Area(Option<String>),
    Tag(String),
    Deadline(String),
    Recurrence(recur::Repeat),
}

impl Hit {
    pub(crate) fn kind(&self) -> Kind {
        match self {
            Hit::Date(_) => Kind::Date,
            Hit::Time(_) => Kind::Time,
            Hit::Duration(_) => Kind::Duration,
            Hit::Priority(_) => Kind::Priority,
            Hit::Project(_) => Kind::Project,
            Hit::Area(_) => Kind::Area,
            Hit::Tag(_) => Kind::Tag,
            Hit::Deadline(_) => Kind::Deadline,
            Hit::Recurrence(_) => Kind::Recurrence,
        }
    }
}

impl Parser for RulesParser {
    fn parse(&self, input: &str, ctx: &Context) -> Parsed {
        let (words, quotes) = words::split(input);
        let mut hits = Vec::new();
        let mut i = 0;
        while i < words.len() {
            let len = words[i..]
                .iter()
                .position(|w| w.quoted)
                .unwrap_or(words.len() - i);
            let run = Run {
                input,
                words: &words[i..i + len],
            };
            let Some((n, hit)) = find(&run, ctx) else {
                i += 1;
                continue;
            };
            hits.push((words[i].start, words[i + n - 1].end, hit));
            i += n;
        }
        assemble(input, ctx, &quotes, hits)
    }
}

/// Builds the result from the hits, in input order, as `(start, end, hit)` byte spans.
pub(crate) fn assemble(
    input: &str,
    ctx: &Context,
    quotes: &[usize],
    hits: Vec<(usize, usize, Hit)>,
) -> Parsed {
    let mut out = Parsed::default();
    let mut acc = Values::default();
    for (start, end, hit) in hits {
        out.tokens.push(Token {
            start,
            end,
            kind: hit.kind(),
            text: input[start..end].to_string(),
        });
        acc.apply(hit, &mut out);
    }
    let day = acc.date.or(acc.first).or(acc.time.map(|_| ctx.today));
    out.scheduled = day.map(|d| stamp(d, acc.time));
    out.title = title(input, &out.tokens, quotes);
    out
}

/// The values that combine into `scheduled` once all tokens are known.
#[derive(Default)]
struct Values {
    date: Option<NaiveDate>,
    time: Option<NaiveTime>,
    first: Option<NaiveDate>,
}

impl Values {
    fn apply(&mut self, hit: Hit, out: &mut Parsed) {
        match hit {
            Hit::Date(d) => self.date = self.date.or(Some(d)),
            Hit::Time(t) => self.time = self.time.or(Some(t)),
            Hit::Duration(m) => out.estimate_min = out.estimate_min.or(Some(m)),
            Hit::Priority(p) => out.priority = out.priority.or(Some(p)),
            Hit::Project(p) => out.project = out.project.take().or(p),
            Hit::Area(a) => out.area = out.area.take().or(a),
            Hit::Deadline(d) => out.deadline = out.deadline.take().or(Some(d)),
            Hit::Tag(t) if !out.tags.contains(&t) => out.tags.push(t),
            Hit::Tag(_) => {}
            Hit::Recurrence(r) if out.recurrence.is_none() => {
                self.first = Some(r.first);
                out.recurrence = Some(r.recurrence);
            }
            Hit::Recurrence(_) => {}
        }
    }
}

/// The first matcher that fits at the start of `run`, with the number of words it used.
pub(crate) fn find(run: &Run, ctx: &Context) -> Option<(usize, Hit)> {
    let today = ctx.today;
    if let Some((n, r)) = recur::repeat(run, today) {
        return Some((n, Hit::Recurrence(r)));
    }
    if let Some((n, d)) = deadline(run, today) {
        return Some((n, Hit::Deadline(d)));
    }
    if let Some((n, d)) = when::date(run, today) {
        return Some((n, Hit::Date(d)));
    }
    if let Some((n, t)) = when::time(run) {
        return Some((n, Hit::Time(t)));
    }
    let w = run.raw(0)?;
    let hit = if let Some(m) = marks::duration(w) {
        Hit::Duration(m)
    } else if let Some(p) = marks::priority(w) {
        Hit::Priority(p)
    } else if let Some(t) = marks::tag(w) {
        Hit::Tag(t)
    } else if let Some(p) = marks::named(w, '+', &ctx.projects) {
        Hit::Project(p)
    } else {
        Hit::Area(marks::named(w, '@', &ctx.areas)?)
    };
    Some((1, hit))
}

/// `due <date> [time]` or `by <date> [time]`.
fn deadline(run: &Run, today: NaiveDate) -> Option<(usize, String)> {
    if !matches!(run.low(0)?.as_str(), "due" | "by") {
        return None;
    }
    let (n, d) = when::date(&run.skip(1), today)?;
    let (m, t) = when::time(&run.skip(1 + n)).map_or((0, None), |(m, t)| (m, Some(t)));
    Some((1 + n + m, stamp(d, t)))
}

/// A date as `YYYY-MM-DD`, or with a time as `YYYY-MM-DDTHH:MM` (the 025 formats).
fn stamp(d: NaiveDate, t: Option<NaiveTime>) -> String {
    match t {
        Some(t) => d.and_time(t).format("%Y-%m-%dT%H:%M").to_string(),
        None => d.format("%Y-%m-%d").to_string(),
    }
}

/// The input without its tokens and paired quotes, whitespace collapsed.
fn title(input: &str, tokens: &[Token], quotes: &[usize]) -> String {
    let kept: String = input
        .char_indices()
        .filter(|(i, _)| {
            !quotes.contains(i) && !tokens.iter().any(|t| (t.start..t.end).contains(i))
        })
        .map(|(_, c)| c)
        .collect();
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}
