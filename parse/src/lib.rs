//! Smart-add parsing: one input line becomes a title plus typed tokens (spec §4 Capture, §5).
//!
//! Every parser sits behind [`Parser`], so candidates can be swapped. [`RulesParser`] is the
//! hand-written baseline; [`default_parser`] returns the benchmark winner. The crate does no
//! I/O.

mod rules;
mod tagger;
mod words;

pub use rules::RulesParser;
pub use tagger::{Labeled, TaggerParser};

use chrono::NaiveDate;
use serde::Serialize;

/// The tagger model chosen by the 030 benchmark (`docs/models/nl-parser.md`).
const MODEL: &[u8] = include_bytes!("tagger/model.bin");

/// The parser smart add ships: the benchmark winner, [`TaggerParser`] with the bundled
/// model. Falls back to [`RulesParser`] if the bundled model can't be read.
pub fn default_parser() -> Box<dyn Parser> {
    match TaggerParser::from_bytes(MODEL) {
        Some(tagger) => Box::new(tagger),
        None => Box::new(RulesParser),
    }
}

/// Turns a smart-add line into a [`Parsed`] result. Never fails: text it doesn't understand
/// stays in the title.
pub trait Parser {
    fn parse(&self, input: &str, ctx: &Context) -> Parsed;
}

/// What the parser needs to know about the world: the local date and the known names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub today: NaiveDate,
    pub projects: Vec<String>,
    pub areas: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Date,
    Time,
    Duration,
    Priority,
    Project,
    Area,
    Tag,
    Deadline,
    Recurrence,
}

/// One recognised span of the input. `start..end` are byte offsets and `text` is that slice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
    pub text: String,
}

/// Where the next occurrence of a recurring task is counted from (the 027 `from` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    Scheduled,
    Completion,
}

/// A rule in the 027 RRULE subset, e.g. `FREQ=WEEKLY;BYDAY=SU`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recurrence {
    pub rule: String,
    pub from: Anchor,
}

/// The parse result. `scheduled` and `deadline` use the 025 formats: `YYYY-MM-DD` or the
/// floating local time `YYYY-MM-DDTHH:MM`. `project` and `area` hold the matching name from
/// [`Context`], and `tags` are normalized (no `#`, lowercase) in input order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Parsed {
    pub title: String,
    pub tokens: Vec<Token>,
    pub scheduled: Option<String>,
    pub deadline: Option<String>,
    pub estimate_min: Option<u32>,
    pub priority: Option<u8>,
    pub recurrence: Option<Recurrence>,
    pub project: Option<String>,
    pub area: Option<String>,
    pub tags: Vec<String>,
}
