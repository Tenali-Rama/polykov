//! The view filter query language (spec §3 "Organization", ADR 0015): a pure parser from text
//! such as `type:task priority:<=2 due:<7d tag:work/* -status:done` to a typed [`Query`].
//! [`parse`] needs no database and no clock; relative dates stay symbolic until
//! [`DateValue::resolve`]. [`compile`] turns a [`Query`] into SQL, [`run`] runs it, and
//! `GET /api/query` serves it.

mod compile;
mod parse;
mod routes;

use std::fmt;

use chrono::{Datelike, Days, Months, NaiveDate};

pub(crate) use compile::bind;
pub use compile::{Compiled, FROM, Item, Page, Param, compile, run};
pub use parse::parse;
pub use routes::{DEFAULT_LIMIT, MAX_LIMIT, MAX_QUERY_CHARS, routes};

/// A parsed query: every term must match (AND).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    pub terms: Vec<Term>,
    /// Which objects to include by their archived flag; `archived:` sets it, default `No`.
    pub archived: Archived,
}

/// One term; `negated` (a leading `-`) keeps the objects it does not match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    pub negated: bool,
    pub filter: Filter,
}

/// What a term matches. A `Vec` is a comma list: any of its items matches (OR).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filter {
    /// A bare word or quoted phrase, matched through search.
    Text(String),
    /// Object types, each one of `objects::validate::TYPES`.
    Type(Vec<String>),
    Tag(Vec<TagPattern>),
    Status(Vec<Status>),
    /// Priorities 1 to 4.
    Priority(Vec<Cmp<u8>>),
    Date(DateField, Vec<Cmp<DateValue>>),
    Project(Vec<Ref>),
    Area(Vec<Ref>),
    Pinned(bool),
}

/// A normalized tag; `subtree` (written `tag/*`) also matches every tag under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPattern {
    pub tag: String,
    pub subtree: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Open,
    Done,
}

/// A comparison of a field against `value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cmp<T> {
    pub op: Op,
    pub value: T,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Lt,
    Le,
    Gt,
    Ge,
}

/// The date fields; `due:` is the task deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateField {
    Due,
    Scheduled,
    Created,
    Updated,
    Completed,
}

/// A date operand. `today`/`tomorrow`/`yesterday` are `Relative` 0/1/-1 days.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateValue {
    /// The field is unset; only with [`Op::Eq`].
    None,
    Absolute(NaiveDate),
    Relative(i32, Unit),
}

/// The unit of a relative offset: `d`, `w`, `m`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Day,
    Week,
    Month,
}

/// A project or area: by id, or by its title (written in quotes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ref {
    Id(i64),
    Title(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archived {
    No,
    Yes,
    Any,
}

/// Why a query does not parse; `offset` is the char index in the input where the fault is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} (at {})", self.message, self.offset)
    }
}

impl std::error::Error for ParseError {}

impl DateValue {
    /// The date this value means when the local date is `today`; `None` for [`DateValue::None`]
    /// or when the date leaves the years 0000 to 9999, where `YYYY-MM-DD` text compares as
    /// dates do. A month offset past the month's end clamps.
    pub fn resolve(self, today: NaiveDate) -> Option<NaiveDate> {
        self.resolve_any(today)
            .filter(|date| (0..=9999).contains(&date.year()))
    }

    fn resolve_any(self, today: NaiveDate) -> Option<NaiveDate> {
        match self {
            DateValue::None => None,
            DateValue::Absolute(date) => Some(date),
            DateValue::Relative(n, unit) => {
                let magnitude = n.unsigned_abs();
                let days = |k: u32| Days::new(u64::from(k));
                match (unit, n < 0) {
                    (Unit::Day, false) => today.checked_add_days(days(magnitude)),
                    (Unit::Day, true) => today.checked_sub_days(days(magnitude)),
                    (Unit::Week, false) => today.checked_add_days(days(magnitude.checked_mul(7)?)),
                    (Unit::Week, true) => today.checked_sub_days(days(magnitude.checked_mul(7)?)),
                    (Unit::Month, false) => today.checked_add_months(Months::new(magnitude)),
                    (Unit::Month, true) => today.checked_sub_months(Months::new(magnitude)),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
