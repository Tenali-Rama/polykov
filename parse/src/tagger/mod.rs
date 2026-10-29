//! Candidate: an averaged-perceptron token tagger. It labels each word `O`, `B-kind` or
//! `I-kind` (greedy, left to right), and each tagged span goes through the rules matchers,
//! which turn it into a value. A span they can't read stays in the title.

mod features;

use std::collections::HashMap;

use crate::rules;
use crate::words::{self, Run};
use crate::{Context, Kind, Parsed, Parser, RulesParser};

const KINDS: [Kind; 9] = [
    Kind::Date,
    Kind::Time,
    Kind::Duration,
    Kind::Priority,
    Kind::Project,
    Kind::Area,
    Kind::Tag,
    Kind::Deadline,
    Kind::Recurrence,
];
/// `O`, then `B-kind` and `I-kind` for each of [`KINDS`].
const LABELS: usize = 1 + 2 * KINDS.len();
/// The stored model's header; bump the version when features or labels change.
const MAGIC: &[u8] = b"polykov-tagger-1\n";
/// One stored feature: its key and a weight per label.
const ROW_BYTES: usize = 8 + 4 * LABELS;

/// A training line: the input, its context, and per word `None` (not a token) or the kind
/// and whether the word starts the token.
#[derive(Debug, Clone)]
pub struct Labeled {
    pub input: String,
    pub ctx: Context,
    pub labels: Vec<Option<(Kind, bool)>>,
}

#[derive(Debug, Clone, Default)]
pub struct TaggerParser {
    weights: HashMap<u64, [f32; LABELS]>,
}

impl TaggerParser {
    /// Trains for `epochs` passes over `data` in a fixed shuffled order, then averages.
    pub fn train(data: &[Labeled], epochs: usize) -> Self {
        let mut t = Training::default();
        let mut order: Vec<usize> = (0..data.len()).collect();
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        for _ in 0..epochs {
            for i in (1..order.len()).rev() {
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                order.swap(i, (seed >> 33) as usize % (i + 1));
            }
            for &i in &order {
                t.learn(&data[i]);
            }
        }
        t.average()
    }

    /// The model as stored in `model.bin`: `MAGIC`, then per feature (sorted by key) the
    /// key and its weight row, little-endian.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut keys: Vec<&u64> = self.weights.keys().collect();
        keys.sort();
        let mut out = MAGIC.to_vec();
        for k in keys {
            out.extend(k.to_le_bytes());
            for w in self.weights[k] {
                out.extend(w.to_le_bytes());
            }
        }
        out
    }

    /// Reads [`TaggerParser::to_bytes`] output; `None` if it is not a model.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let body = bytes.strip_prefix(MAGIC)?;
        let (rows, rest) = body.as_chunks::<ROW_BYTES>();
        if !rest.is_empty() {
            return None;
        }
        let weights = rows
            .iter()
            .map(|row| {
                let (key, ws) = row.split_first_chunk::<8>()?;
                let (ws, _) = ws.as_chunks::<4>();
                let ws: [f32; LABELS] = ws
                    .iter()
                    .map(|w| f32::from_le_bytes(*w))
                    .collect::<Vec<_>>()
                    .try_into()
                    .ok()?;
                Some((u64::from_le_bytes(*key), ws))
            })
            .collect::<Option<_>>()?;
        Some(TaggerParser { weights })
    }

    /// The model's size in bytes, as stored.
    pub fn model_bytes(&self) -> usize {
        MAGIC.len() + self.weights.len() * ROW_BYTES
    }

    fn score(&self, feats: &[u64]) -> [f32; LABELS] {
        let mut s = [0.0; LABELS];
        for row in feats.iter().filter_map(|f| self.weights.get(f)) {
            for (a, b) in s.iter_mut().zip(row) {
                *a += b;
            }
        }
        s
    }
}

impl Parser for TaggerParser {
    fn parse(&self, input: &str, ctx: &Context) -> Parsed {
        let (words, quotes) = words::split(input);
        let lows: Vec<String> = words
            .iter()
            .map(|w| input[w.start..w.end].to_lowercase())
            .collect();
        let starts: Vec<usize> = words.iter().map(|w| w.start).collect();
        let rule = features::rule_labels(&starts, &RulesParser.parse(input, ctx).tokens);
        let mut labels = Vec::with_capacity(words.len());
        let mut prev = 0;
        for (i, w) in words.iter().enumerate() {
            let label = if w.quoted {
                0
            } else {
                argmax(&self.score(&features::of(&lows, &rule, i, prev)))
            };
            labels.push(label);
            prev = label;
        }
        let mut hits = Vec::new();
        let mut i = 0;
        while i < words.len() {
            let Some(kind) = kind_of(labels[i]) else {
                i += 1;
                continue;
            };
            let n = 1 + labels[i + 1..]
                .iter()
                .take_while(|l| **l == inside(kind))
                .count();
            let run = Run {
                input,
                words: &words[i..i + n],
            };
            match rules::find(&run, ctx) {
                Some((m, hit)) if m == n && hit.kind() == kind => {
                    hits.push((words[i].start, words[i + n - 1].end, hit));
                }
                _ => {}
            }
            i += n;
        }
        rules::assemble(input, ctx, &quotes, hits)
    }
}

/// Weights with lazy averaging state: running totals and the step each was last updated.
#[derive(Default)]
struct Training {
    step: u32,
    rows: HashMap<u64, [(f32, f32, u32); LABELS]>,
}

impl Training {
    fn learn(&mut self, ex: &Labeled) {
        let (words, _) = words::split(&ex.input);
        let lows: Vec<String> = words
            .iter()
            .map(|w| ex.input[w.start..w.end].to_lowercase())
            .collect();
        let starts: Vec<usize> = words.iter().map(|w| w.start).collect();
        let rule = features::rule_labels(&starts, &RulesParser.parse(&ex.input, &ex.ctx).tokens);
        let mut prev = 0;
        for (i, gold) in ex.labels.iter().enumerate().take(words.len()) {
            self.step += 1;
            let gold = encode(*gold);
            let feats = features::of(&lows, &rule, i, prev);
            let mut s = [0.0; LABELS];
            for row in feats.iter().filter_map(|f| self.rows.get(f)) {
                for (a, (w, _, _)) in s.iter_mut().zip(row) {
                    *a += w;
                }
            }
            let guess = argmax(&s);
            if guess != gold {
                for f in &feats {
                    self.bump(*f, gold, 1.0);
                    self.bump(*f, guess, -1.0);
                }
            }
            prev = guess;
        }
    }

    fn bump(&mut self, f: u64, label: usize, by: f32) {
        let step = self.step;
        let cell = &mut self.rows.entry(f).or_insert([(0.0, 0.0, 0); LABELS])[label];
        cell.1 += cell.0 * (step - cell.2) as f32;
        cell.0 += by;
        cell.2 = step;
    }

    fn average(self) -> TaggerParser {
        let step = self.step.max(1);
        let weights = self
            .rows
            .into_iter()
            .map(|(f, row)| {
                let avg =
                    row.map(|(w, total, last)| (total + w * (step - last) as f32) / step as f32);
                (f, avg)
            })
            .filter(|(_, row)| row.iter().any(|w| w.abs() > 1e-6))
            .collect();
        TaggerParser { weights }
    }
}

fn encode(label: Option<(Kind, bool)>) -> usize {
    label.map_or(0, |(kind, begin)| {
        let k = KINDS.iter().position(|x| *x == kind).unwrap_or(0);
        1 + 2 * k + usize::from(!begin)
    })
}

/// The kind a label starts a span of; an `I-` label with no `B-` before it starts one too.
fn kind_of(label: usize) -> Option<Kind> {
    (label > 0).then(|| KINDS[(label - 1) / 2])
}

fn inside(kind: Kind) -> usize {
    encode(Some((kind, false)))
}

fn argmax(s: &[f32; LABELS]) -> usize {
    (0..LABELS).fold(0, |best, i| if s[i] > s[best] { i } else { best })
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn ctx() -> Context {
        Context {
            today: NaiveDate::from_ymd_opt(2026, 10, 30).unwrap(),
            projects: vec![],
            areas: vec![],
        }
    }

    fn ex(input: &str, labels: &[Option<(Kind, bool)>]) -> Labeled {
        Labeled {
            input: input.into(),
            ctx: ctx(),
            labels: labels.to_vec(),
        }
    }

    /// "sun" is a date after "run" but not before "cream".
    fn tiny() -> TaggerParser {
        let date = Some((Kind::Date, true));
        let data = [
            ex("buy sun cream", &[None, None, None]),
            ex("grocery run sun", &[None, None, date]),
            ex("get sun cream sat", &[None, None, None, date]),
            ex("long run sun", &[None, None, date]),
        ];
        TaggerParser::train(&data, 5)
    }

    #[test]
    fn learns_context_the_rules_ignore() {
        let t = tiny();
        let p = t.parse("buy sun cream", &ctx());
        assert_eq!((p.title.as_str(), p.scheduled), ("buy sun cream", None));
        let p = t.parse("grocery run sun", &ctx());
        assert_eq!(p.title, "grocery run");
        assert_eq!(p.scheduled.as_deref(), Some("2026-11-01"));
    }

    #[test]
    fn quoted_words_are_never_tagged() {
        let p = tiny().parse(r#"grocery run "sun""#, &ctx());
        assert_eq!((p.title.as_str(), p.tokens.len()), ("grocery run sun", 0));
    }

    #[test]
    fn model_round_trips_through_bytes() {
        let t = tiny();
        let bytes = t.to_bytes();
        assert_eq!(bytes.len(), t.model_bytes());
        let back = TaggerParser::from_bytes(&bytes).unwrap();
        assert_eq!(back.weights, t.weights);
        assert_eq!(back.to_bytes(), bytes, "stored order is stable");
    }

    #[test]
    fn rejects_bytes_that_are_not_a_model() {
        let bytes = tiny().to_bytes();
        assert!(TaggerParser::from_bytes(&bytes[..bytes.len() - 1]).is_none());
        assert!(TaggerParser::from_bytes(b"not a model").is_none());
        assert!(TaggerParser::from_bytes(&[]).is_none());
    }

    #[test]
    fn bundled_model_loads() {
        let t = TaggerParser::from_bytes(include_bytes!("model.bin")).unwrap();
        assert!(!t.weights.is_empty());
    }
}
