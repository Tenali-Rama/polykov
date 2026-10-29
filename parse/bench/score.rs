//! The NL parser benchmark: loading `cases.jsonl` and scoring a parse against a case.
//! Shared by `examples/bench.rs` and `tests/bench.rs` through `#[path]`.

use chrono::NaiveDate;
use polykov_parse::{Anchor, Context, Kind, Parsed};
use serde::Deserialize;

/// One benchmark line. `slice` is `casual`, `standard`, `cold` or `long`.
#[derive(Debug, Deserialize)]
pub struct Case {
    pub slice: String,
    pub input: String,
    pub today: String,
    pub projects: Vec<String>,
    pub areas: Vec<String>,
    pub expected: Expected,
}

#[derive(Debug, Deserialize)]
pub struct Expected {
    pub title: String,
    pub scheduled: Option<String>,
    pub deadline: Option<String>,
    pub estimate_min: Option<u32>,
    pub priority: Option<u8>,
    pub recurrence: Option<Rule>,
    pub project: Option<String>,
    pub area: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Rule {
    pub rule: String,
    pub from: String,
}

/// The scored fields, in the order [`Expected::hits`] reports them.
pub const FIELDS: [&str; 9] = [
    "title",
    "scheduled",
    "deadline",
    "estimate",
    "priority",
    "recurrence",
    "project",
    "area",
    "tags",
];

/// A `train.jsonl` line: per word `O`, `B-<kind>` or `I-<kind>`.
#[derive(Debug, Deserialize)]
pub struct TrainCase {
    #[serde(flatten)]
    pub case: Case,
    pub labels: Vec<String>,
}

impl TrainCase {
    pub fn labels(&self) -> Vec<Option<(Kind, bool)>> {
        self.labels
            .iter()
            .map(|l| {
                let (begin, name) = match l.split_once('-') {
                    Some(("B", n)) => (true, n),
                    Some((_, n)) => (false, n),
                    None => return None,
                };
                let kind = match name {
                    "date" => Kind::Date,
                    "time" => Kind::Time,
                    "duration" => Kind::Duration,
                    "priority" => Kind::Priority,
                    "project" => Kind::Project,
                    "area" => Kind::Area,
                    "tag" => Kind::Tag,
                    "deadline" => Kind::Deadline,
                    _ => Kind::Recurrence,
                };
                Some((kind, begin))
            })
            .collect()
    }
}

impl Case {
    pub fn context(&self) -> Context {
        Context {
            today: NaiveDate::parse_from_str(&self.today, "%Y-%m-%d").expect("case date"),
            projects: self.projects.clone(),
            areas: self.areas.clone(),
        }
    }
}

impl Expected {
    /// Whether each of [`FIELDS`] matches `p` exactly.
    pub fn hits(&self, p: &Parsed) -> [bool; 9] {
        let rec = p.recurrence.as_ref().map(|r| Rule {
            rule: r.rule.clone(),
            from: match r.from {
                Anchor::Scheduled => "scheduled".into(),
                Anchor::Completion => "completion".into(),
            },
        });
        [
            self.title == p.title,
            self.scheduled == p.scheduled,
            self.deadline == p.deadline,
            self.estimate_min == p.estimate_min,
            self.priority == p.priority,
            self.recurrence == rec,
            self.project == p.project,
            self.area == p.area,
            self.tags == p.tags,
        ]
    }

    /// A line with nothing to extract: every token a parser finds there is a false one.
    pub fn no_token(&self) -> bool {
        self.scheduled.is_none()
            && self.deadline.is_none()
            && self.estimate_min.is_none()
            && self.priority.is_none()
            && self.recurrence.is_none()
            && self.project.is_none()
            && self.area.is_none()
            && self.tags.is_empty()
    }
}

/// Reads a JSON-lines file under `parse/bench/`.
pub fn load<T: serde::de::DeserializeOwned>(name: &str) -> Vec<T> {
    let path = format!("{}/bench/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{path}: {e}: {l}")))
        .collect()
}
