//! The SQL behind the GTD lists: which open tasks each list holds, in what order, and the
//! sub-task counts. Rows come back through `tasks::store::get_many`.

use rusqlite::{Connection, ToSql, params};
use serde::Serialize;

use crate::{
    db::{self, Db},
    tasks::store::{self as tasks, Task},
};

/// A GTD list of `GET /api/lists/{name}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum List {
    Inbox,
    Today,
    Upcoming,
    Someday,
    Anytime,
    All,
}

impl List {
    /// The list called `name`, or `None` for an unknown name.
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "inbox" => List::Inbox,
            "today" => List::Today,
            "upcoming" => List::Upcoming,
            "someday" => List::Someday,
            "anytime" => List::Anytime,
            "all" => List::All,
            _ => return None,
        })
    }

    /// The `WHERE` rule and `ORDER BY` over the `open_tasks` CTE; `?1` is today.
    fn sql(self) -> (String, &'static str) {
        match self {
            List::Inbox => (INBOX.into(), "created DESC, id DESC"),
            List::Today => (
                "substr(scheduled, 1, 10) <= ?1 OR substr(deadline, 1, 10) <= ?1".into(),
                "priority, deadline IS NULL, deadline, scheduled IS NULL, scheduled, id",
            ),
            List::Upcoming => (
                "substr(scheduled, 1, 10) > ?1 \
                 OR (scheduled IS NULL AND substr(deadline, 1, 10) > ?1)"
                    .into(),
                "substr(COALESCE(scheduled, deadline), 1, 10), priority, id",
            ),
            List::Someday => (
                "someday = 1 AND scheduled IS NULL AND parent IS NULL".into(),
                "priority, id",
            ),
            List::Anytime => (
                format!(
                    "someday = 0 AND NOT ({INBOX}) AND parent IS NULL \
                     AND (scheduled IS NULL OR substr(scheduled, 1, 10) <= ?1)"
                ),
                "priority, id",
            ),
            List::All => ("parent IS NULL".into(), "priority, id"),
        }
    }
}

/// The inbox rule: not filed, dated or deferred.
const INBOX: &str = "project IS NULL AND area IS NULL AND parent IS NULL \
                     AND scheduled IS NULL AND deadline IS NULL AND someday = 0";

/// Live, non-archived, open tasks with their references resolved; a reference to a trashed
/// object reads as `NULL`.
const OPEN_TASKS: &str = "WITH open_tasks AS (SELECT o.id, o.created, \
     COALESCE(t.priority, 4) AS priority, t.scheduled, t.deadline, \
     COALESCE(t.someday, 0) AS someday, \
     (SELECT r.id FROM objects r WHERE r.id = t.parent_id AND r.deleted_at IS NULL) AS parent, \
     (SELECT r.id FROM objects r WHERE r.id = t.project_id AND r.deleted_at IS NULL) AS project, \
     (SELECT r.id FROM objects r WHERE r.id = t.area_id AND r.deleted_at IS NULL) AS area \
     FROM objects o LEFT JOIN tasks t ON t.object_id = o.id \
     WHERE o.type = 'task' AND o.deleted_at IS NULL AND o.archived = 0 \
     AND COALESCE(t.status, 'open') = 'open')";

/// The live, non-archived children of the task bound to `?1`.
const CHILDREN: &str = "FROM objects o JOIN tasks t ON t.object_id = o.id \
     WHERE t.parent_id = ?1 AND o.type = 'task' AND o.deleted_at IS NULL AND o.archived = 0";

/// A validated list request. `today` is `YYYY-MM-DD`; `project`/`area` filter by those ids.
#[derive(Debug)]
pub struct Query {
    pub list: List,
    pub today: String,
    pub project: Option<i64>,
    pub area: Option<i64>,
    pub limit: u32,
    pub offset: i64,
}

/// Sub-task counts of a row.
#[derive(Debug, Serialize)]
pub struct Counts {
    pub open: i64,
    pub total: i64,
}

/// A task plus its sub-task counts.
#[derive(Debug, Serialize)]
pub struct Row {
    #[serde(flatten)]
    pub task: Task,
    pub subtasks: Counts,
}

/// One page of a list and the number of matches across all pages.
#[derive(Debug, Serialize)]
pub struct Page {
    pub tasks: Vec<Row>,
    pub total: i64,
}

/// The page of `q.list` that `q` asks for.
pub fn list(db: &Db, q: &Query) -> Result<Page, db::Error> {
    let (rule, order) = q.list.sql();
    let from = format!(
        "FROM open_tasks WHERE ({rule}) AND (?2 IS NULL OR project = ?2) \
         AND (?3 IS NULL OR area = ?3 \
              OR project IN (SELECT p.object_id FROM projects p \
                  JOIN objects a ON a.id = p.area_id AND a.deleted_at IS NULL \
                  WHERE p.area_id = ?3))"
    );
    let (ids, total) = {
        let conn = db.conn()?;
        let total = conn
            .query_row(
                &format!("{OPEN_TASKS} SELECT COUNT(*) {from}"),
                params![q.today, q.project, q.area],
                |r| r.get(0),
            )
            .map_err(db::Error::Sqlite)?;
        let sql = format!("{OPEN_TASKS} SELECT id {from} ORDER BY {order} LIMIT ?4 OFFSET ?5");
        let args = params![q.today, q.project, q.area, q.limit, q.offset];
        (ids(&conn, &sql, args)?, total)
    };
    Ok(Page {
        tasks: rows(db, &ids)?,
        total,
    })
}

/// The live, non-archived children of task `parent`: open first, then done, each by id.
pub fn subtasks(db: &Db, parent: i64) -> Result<Vec<Row>, db::Error> {
    let sql = format!("SELECT o.id {CHILDREN} ORDER BY t.status = 'done', o.id");
    let ids = ids(&*db.conn()?, &sql, params![parent])?;
    rows(db, &ids)
}

fn ids(conn: &Connection, sql: &str, args: &[&dyn ToSql]) -> Result<Vec<i64>, db::Error> {
    let mut stmt = conn.prepare(sql).map_err(db::Error::Sqlite)?;
    stmt.query_map(args, |r| r.get(0))
        .map_err(db::Error::Sqlite)?
        .collect::<Result<_, _>>()
        .map_err(db::Error::Sqlite)
}

/// The tasks among `ids`, in order, each with its sub-task counts.
fn rows(db: &Db, ids: &[i64]) -> Result<Vec<Row>, db::Error> {
    let tasks = tasks::get_many(db, ids)?;
    let conn = db.conn()?;
    let sql = format!("SELECT COUNT(*) FILTER (WHERE t.status = 'open'), COUNT(*) {CHILDREN}");
    tasks
        .into_iter()
        .map(|task| {
            let subtasks = conn
                .query_row(&sql, [task.object.id], |r| {
                    Ok(Counts {
                        open: r.get(0)?,
                        total: r.get(1)?,
                    })
                })
                .map_err(db::Error::Sqlite)?;
            Ok(Row { task, subtasks })
        })
        .collect()
}
