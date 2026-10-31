//! A parsed [`Query`] to SQL, and running it: the `WHERE` rule over `objects` (left-joined
//! with `tasks t`), with every value bound as a parameter.

use chrono::NaiveDate;
use rusqlite::{Connection, params_from_iter, types::Value};
use serde::Serialize;

use super::{Archived, Cmp, DateField, DateValue, Filter, Op, Query, Ref, Status, TagPattern};
use crate::{
    db::{self, Db},
    http::ApiError,
    objects::store::{self as objects, Object},
    search, tags,
    tasks::store::{self as tasks, Task},
};

/// The `FROM` clause that [`Compiled::sql_where`] is written against.
pub const FROM: &str = "objects LEFT JOIN tasks t ON t.object_id = objects.id";

/// A compiled query: `sql_where` refers to `?1..?n`, the n `params` in order.
#[derive(Debug, Clone, PartialEq)]
pub struct Compiled {
    pub sql_where: String,
    pub params: Vec<Param>,
}

/// One bound value. `Search` is a text term: [`run`] binds it as the JSON array of the ids
/// that `search::matching_ids` finds for it.
#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    Value(Value),
    Search(String),
}

/// A matching object; a task carries its task fields, as in the tasks API.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Item {
    Task(Box<Task>),
    Object(Object),
}

/// One page of matches and the number of matches across all pages.
#[derive(Debug, Serialize)]
pub struct Page {
    pub objects: Vec<Item>,
    pub total: i64,
}

/// Compiles `query` with relative dates resolved against `today`: live objects (with the
/// query's archived rule) that match every term. A relative date beyond the calendar's range
/// is a `400 date is out of range`.
pub fn compile(query: &Query, today: NaiveDate) -> Result<Compiled, ApiError> {
    let mut c = Compiled {
        sql_where: "objects.deleted_at IS NULL".into(),
        params: Vec::new(),
    };
    match query.archived {
        Archived::No => c.sql_where.push_str(" AND objects.archived = 0"),
        Archived::Yes => c.sql_where.push_str(" AND objects.archived = 1"),
        Archived::Any => {}
    }
    for term in &query.terms {
        let rule = c.filter(&term.filter, today)?;
        // NULL counts as no match, so that a negated term keeps exactly the rest.
        let not = if term.negated { "NOT " } else { "" };
        c.sql_where
            .push_str(&format!(" AND {not}COALESCE(({rule}), 0)"));
    }
    Ok(c)
}

/// The rule of a task-only field: `cond` on the task row of a task, false for other types.
fn task_rule(cond: String) -> String {
    format!("objects.type = 'task' AND ({cond})")
}

impl Compiled {
    /// Binds `value` and returns its placeholder.
    fn bind(&mut self, value: impl Into<Value>) -> String {
        self.params.push(Param::Value(value.into()));
        format!("?{}", self.params.len())
    }

    /// `items` ORed, each built by `rule`.
    fn any<T>(
        &mut self,
        items: &[T],
        mut rule: impl FnMut(&mut Self, &T) -> Result<String, ApiError>,
    ) -> Result<String, ApiError> {
        let rules = items
            .iter()
            .map(|item| rule(self, item).map(|r| format!("({r})")))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rules.join(" OR "))
    }

    fn filter(&mut self, filter: &Filter, today: NaiveDate) -> Result<String, ApiError> {
        Ok(match filter {
            Filter::Text(text) => {
                self.params.push(Param::Search(text.clone()));
                let n = self.params.len();
                format!("objects.id IN (SELECT value FROM json_each(?{n}))")
            }
            Filter::Type(types) => self.any(types, |c, t| {
                Ok(format!("objects.type = {}", c.bind(t.clone())))
            })?,
            Filter::Tag(patterns) => self.any(patterns, |c, p| Ok(c.tag(p)))?,
            Filter::Status(statuses) => task_rule(self.any(statuses, |c, s| {
                let s = match s {
                    Status::Open => "open",
                    Status::Done => "done",
                };
                Ok(format!(
                    "COALESCE(t.status, 'open') = {}",
                    c.bind(s.to_owned())
                ))
            })?),
            Filter::Priority(cmps) => task_rule(self.any(cmps, |c, cmp| {
                let value = c.bind(i64::from(cmp.value));
                Ok(format!("COALESCE(t.priority, 4) {} {value}", op(cmp.op)))
            })?),
            Filter::Date(field, cmps) => self.any(cmps, |c, cmp| c.date(*field, cmp, today))?,
            Filter::Project(refs) => {
                let rule = self.any(refs, |c, r| Ok(c.reference("project", r)))?;
                task_rule(format!(
                    "t.project_id IN (SELECT id FROM objects WHERE {rule})"
                ))
            }
            Filter::Area(refs) => {
                let rule = self.any(refs, |c, r| Ok(c.reference("area", r)))?;
                let areas = format!("SELECT id FROM objects WHERE {rule}");
                // Live projects in one of the areas.
                let projects = format!(
                    "SELECT p.object_id FROM projects p JOIN objects po ON po.id = p.object_id \
                     WHERE po.deleted_at IS NULL AND p.area_id IN ({areas})"
                );
                format!(
                    "(objects.type = 'task' AND (t.area_id IN ({areas}) \
                      OR t.project_id IN ({projects}))) \
                     OR (objects.type = 'project' AND objects.id IN ({projects}))"
                )
            }
            Filter::Pinned(pinned) => format!("objects.pinned = {}", self.bind(*pinned)),
        })
    }

    fn tag(&mut self, pattern: &TagPattern) -> String {
        let n = self.bind(pattern.tag.clone());
        if pattern.subtree {
            tags::store::under(self.params.len())
        } else {
            format!("objects.id IN (SELECT object_id FROM object_tags WHERE tag = {n})")
        }
    }

    /// A live object of type `kind` named by `r` (a title matches ignoring ASCII case).
    fn reference(&mut self, kind: &str, r: &Ref) -> String {
        let kind = self.bind(kind.to_owned());
        let named = match r {
            Ref::Id(id) => format!("id = {}", self.bind(*id)),
            Ref::Title(title) => format!("title = {} COLLATE NOCASE", self.bind(title.clone())),
        };
        format!("deleted_at IS NULL AND type = {kind} AND {named}")
    }

    fn date(
        &mut self,
        field: DateField,
        cmp: &Cmp<DateValue>,
        today: NaiveDate,
    ) -> Result<String, ApiError> {
        // Each column as a `YYYY-MM-DD` date; a date-time task date compares by its date.
        let column = match field {
            DateField::Due => "substr(t.deadline, 1, 10)",
            DateField::Scheduled => "substr(t.scheduled, 1, 10)",
            DateField::Completed => "date(t.completed, 'unixepoch')",
            DateField::Created => "date(objects.created, 'unixepoch')",
            DateField::Updated => "date(objects.updated, 'unixepoch')",
        };
        let rule = if cmp.value == DateValue::None {
            format!("{column} IS NULL")
        } else {
            let date = cmp
                .value
                .resolve(today)
                .ok_or_else(|| ApiError::bad_request("date is out of range"))?;
            let value = self.bind(date.format("%Y-%m-%d").to_string());
            format!("{column} {} {value}", op(cmp.op))
        };
        Ok(match field {
            DateField::Created | DateField::Updated => rule,
            _ => task_rule(rule),
        })
    }
}

fn op(op: Op) -> &'static str {
    match op {
        Op::Eq => "=",
        Op::Lt => "<",
        Op::Le => "<=",
        Op::Gt => ">",
        Op::Ge => ">=",
    }
}

/// The page of live objects matching `query` (relative dates against `today`): pinned
/// first, then `updated` newest first, then `id` highest first, as in the object list. Errors
/// are those of [`compile`], or a store failure.
pub fn run(
    db: &Db,
    query: &Query,
    today: NaiveDate,
    limit: u32,
    offset: i64,
) -> Result<Page, ApiError> {
    let compiled = compile(query, today)?;
    let (rows, total) = {
        let conn = db.conn()?;
        let mut args = bind(&conn, &compiled.params)?;
        let filter = format!("FROM {FROM} WHERE {}", compiled.sql_where);
        let total = conn
            .query_row(
                &format!("SELECT COUNT(*) {filter}"),
                params_from_iter(&args),
                |r| r.get(0),
            )
            .map_err(db::Error::Sqlite)?;
        let n = args.len();
        args.extend([Value::from(i64::from(limit)), Value::from(offset)]);
        let sql = format!(
            "SELECT objects.id, objects.type {filter} \
             ORDER BY objects.pinned DESC, objects.updated DESC, objects.id DESC \
             LIMIT ?{} OFFSET ?{}",
            n + 1,
            n + 2
        );
        let mut stmt = conn.prepare(&sql).map_err(db::Error::Sqlite)?;
        let rows = stmt
            .query_map(params_from_iter(&args), |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(db::Error::Sqlite)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db::Error::Sqlite)?;
        (rows, total)
    };
    let mut items = Vec::with_capacity(rows.len());
    for (id, kind) in rows {
        let item = if kind == "task" {
            tasks::get(db, id)?.map(|t| Item::Task(Box::new(t)))
        } else {
            objects::get(db, id)?.map(Item::Object)
        };
        items.extend(item);
    }
    Ok(Page {
        objects: items,
        total,
    })
}

/// The SQL values of `params`, each text term bound as the JSON array of the ids that
/// `search::matching_ids` finds for it. `run` and `views` both bind through this.
pub(crate) fn bind(conn: &Connection, params: &[Param]) -> Result<Vec<Value>, db::Error> {
    params
        .iter()
        .map(|p| match p {
            Param::Value(v) => Ok(v.clone()),
            Param::Search(text) => {
                let ids = search::matching_ids(conn, text)?;
                Ok(Value::Text(serde_json::Value::from(ids).to_string()))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::parse;

    #[test]
    fn user_values_are_bound_never_formatted() {
        let input = "hello' type:task tag:a'b/* project:\"x' --\" due:<1d -pinned:true";
        let today = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let compiled = compile(&parse(input).unwrap(), today).unwrap();
        for user in ["hello", "x'", "a'b", "2026"] {
            assert!(!compiled.sql_where.contains(user), "{user}");
        }
        let text = |s: &str| Param::Value(Value::Text(s.into()));
        assert_eq!(
            compiled.params,
            [
                Param::Search("hello'".into()),
                text("task"),
                text("a'b"),
                text("project"),
                text("x' --"),
                text("2026-11-02"),
                Param::Value(Value::Integer(1)),
            ]
        );
    }
}
