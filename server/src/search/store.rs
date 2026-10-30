//! Search over the `search` FTS5 index (migration 6): the index narrows the candidates,
//! then every query word is matched in Rust ([`fuzzy::find`]) and the hits are scored.

use std::cmp::Ordering;

use rusqlite::{Connection, Row, params};

use super::fuzzy::{self, Found, MIN_FUZZY, MIN_INDEXED};
use crate::db::{self, Db};

/// Weight of a match in each indexed column: title, tags, property values, comments.
const WEIGHTS: [f64; 4] = [4.0, 2.0, 1.0, 1.0];
/// The indexed columns, in [`WEIGHTS`] order.
const COLUMNS: [&str; 4] = ["title", "tags", "props", "comments"];

/// A validated search.
#[derive(Debug)]
pub struct Query {
    /// Trimmed, 1 to `MAX_QUERY_CHARS` characters.
    pub text: String,
    /// Only objects of this type, if given.
    pub r#type: Option<String>,
    pub limit: u32,
    pub offset: usize,
}

/// One matching object.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: i64,
    pub score: f64,
    /// The best-matching column's text with the matches in `[` `]`.
    pub snippet: String,
}

/// One page of hits, best first, and the number of hits across all pages.
#[derive(Debug)]
pub struct Page {
    pub hits: Vec<Hit>,
    pub total: usize,
}

/// A query word, folded, and whether it may match with a typo.
struct Word {
    chars: Vec<char>,
    typos: bool,
}

/// A candidate that matched every word: its score and the column to take the snippet from.
struct Scored {
    id: i64,
    score: f64,
    column: usize,
}

/// Runs `query` and returns the requested page.
pub fn search(db: &Db, query: &Query) -> Result<Page, db::Error> {
    let conn = db.conn()?;
    let kind = query.r#type.as_deref();
    let page = || -> rusqlite::Result<Page> {
        let words = words(&conn, &query.text, kind)?;
        let ranked = rank(&conn, &words, kind)?;
        let hits = ranked
            .iter()
            .skip(query.offset)
            .take(query.limit as usize)
            .map(|s| hit(&conn, &words, s))
            .collect::<rusqlite::Result<_>>()?;
        Ok(Page {
            hits,
            total: ranked.len(),
        })
    };
    page().map_err(db::Error::Sqlite)
}

/// The ids of every live object (archived and completed included) matching `text` with the
/// rules of `GET /api/search`, best match first. Empty when `text` has no words. For other
/// modules that filter by text, e.g. inside their own query with
/// `id IN (SELECT value FROM json_each(?))`.
pub fn matching_ids(conn: &Connection, text: &str) -> Result<Vec<i64>, db::Error> {
    let ranked = words(conn, text, None)
        .and_then(|words| rank(conn, &words, None))
        .map_err(db::Error::Sqlite)?;
    Ok(ranked.into_iter().map(|s| s.id).collect())
}

/// The words of `text`. A word of [`MIN_FUZZY`]+ characters may match with a typo only if
/// it occurs exactly nowhere (in objects of type `kind`, if given): exact matches are what
/// the owner meant, and the halves of a common word would make most rows candidates.
fn words(conn: &Connection, text: &str, kind: Option<&str>) -> rusqlite::Result<Vec<Word>> {
    fuzzy::words(text)
        .into_iter()
        .map(|chars| {
            let typos = chars.len() >= MIN_FUZZY && !occurs(conn, &chars, kind)?;
            Ok(Word { chars, typos })
        })
        .collect()
}

/// Whether `word` occurs exactly in the index (in an object of type `kind`, if given).
fn occurs(conn: &Connection, word: &[char], kind: Option<&str>) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM search JOIN objects o ON o.id = search.rowid \
         WHERE search MATCH ?1 AND (?2 IS NULL OR o.type = ?2))",
        params![quote(word), kind],
        |r| r.get(0),
    )
}

/// The candidates that match every word, best first, then newest id first.
fn rank(conn: &Connection, words: &[Word], kind: Option<&str>) -> rusqlite::Result<Vec<Scored>> {
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let mut ranked = match match_expr(words) {
        Some(expr) => indexed(conn, words, &expr, kind)?,
        None => titles(conn, words, kind)?,
    };
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(Ordering::Equal)
            .then(b.id.cmp(&a.id))
    });
    Ok(ranked)
}

/// The FTS5 candidates for `expr` (of type `kind`, if given) that match every word in some
/// column.
fn indexed(
    conn: &Connection,
    words: &[Word],
    expr: &str,
    kind: Option<&str>,
) -> rusqlite::Result<Vec<Scored>> {
    let filter = if kind.is_some() {
        "JOIN objects o ON o.id = search.rowid WHERE o.type = ?2 AND"
    } else {
        "WHERE ?2 IS NULL AND"
    };
    let mut stmt = conn.prepare(&format!(
        "SELECT search.rowid, {} FROM search {filter} search MATCH ?1",
        COLUMNS.map(|c| format!("search.{c}")).join(", "),
    ))?;
    let mut rows = stmt.query(params![expr, kind])?;
    let mut ranked = Vec::new();
    let mut texts: [Vec<char>; 4] = Default::default();
    while let Some(row) = rows.next()? {
        columns(row, &mut texts)?;
        if let Some((score, column)) = score(words, &texts) {
            ranked.push(Scored {
                id: row.get(0)?,
                score,
                column,
            });
        }
    }
    Ok(ranked)
}

/// The live objects (of type `kind`, if given) whose title has every word at the start of
/// one of its words. For queries with no word the index can look up: matching such words
/// anywhere in every column would score every row and break the budget (ADR 0014).
fn titles(conn: &Connection, words: &[Word], kind: Option<&str>) -> rusqlite::Result<Vec<Scored>> {
    let mut stmt = conn.prepare(
        "SELECT id, title FROM objects WHERE deleted_at IS NULL AND (?1 IS NULL OR type = ?1)",
    )?;
    let mut rows = stmt.query([kind])?;
    let mut ranked = Vec::new();
    'rows: while let Some(row) = rows.next()? {
        let title = row.get_ref(1)?.as_str()?;
        let mut score = 0.0;
        for word in words {
            match fuzzy::word_start(&word.chars, title) {
                Some(quality) => score += quality * WEIGHTS[0],
                None => continue 'rows,
            }
        }
        ranked.push(Scored {
            id: row.get(0)?,
            score,
            column: 0,
        });
    }
    Ok(ranked)
}

/// `word` as an FTS5 phrase. Words are letters and digits only, so no FTS syntax gets in.
fn quote(word: &[char]) -> String {
    format!("\"{}\"", word.iter().collect::<String>())
}

/// The FTS5 query that every match satisfies: for each indexable word, the word itself or,
/// if it may have a typo, either half of it (one typo leaves one half intact). `None` when
/// no word is indexable.
fn match_expr(words: &[Word]) -> Option<String> {
    let groups: Vec<String> = words
        .iter()
        .filter(|w| w.chars.len() >= MIN_INDEXED)
        .map(|w| {
            let mut terms = vec![quote(&w.chars)];
            if w.typos {
                let (a, b) = w.chars.split_at(w.chars.len() / 2);
                terms.extend([quote(a), quote(b)]);
            }
            format!("({})", terms.join(" OR "))
        })
        .collect();
    (!groups.is_empty()).then(|| groups.join(" AND "))
}

/// Reads the four indexed columns of `row` (after its id) into `columns`, case-folded.
fn columns(row: &Row, columns: &mut [Vec<char>; 4]) -> rusqlite::Result<()> {
    for (i, column) in columns.iter_mut().enumerate() {
        column.clear();
        column.extend(row.get_ref(i + 1)?.as_str()?.chars().map(fuzzy::fold));
    }
    Ok(())
}

/// Every word's best weighted match across the columns, summed, and the column of the
/// strongest one; `None` if a word matches nowhere.
fn score(words: &[Word], columns: &[Vec<char>; 4]) -> Option<(f64, usize)> {
    let mut total = 0.0;
    let mut top = (0.0, 0);
    for word in words {
        let best = (0..columns.len())
            .filter_map(|c| {
                fuzzy::find(&word.chars, &columns[c], word.typos)
                    .map(|f| (f.quality * WEIGHTS[c], c))
            })
            .max_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(Ordering::Equal)
                    .then(b.1.cmp(&a.1))
            })?;
        total += best.0;
        if best.0 > top.0 {
            top = best;
        }
    }
    Some((total, top.1))
}

/// The page entry for `scored`, with its snippet.
fn hit(conn: &Connection, words: &[Word], scored: &Scored) -> rusqlite::Result<Hit> {
    let text: String = conn.query_row(
        &format!(
            "SELECT {} FROM search WHERE rowid = ?1",
            COLUMNS[scored.column]
        ),
        [scored.id],
        |r| r.get(0),
    )?;
    let folded: Vec<char> = text.chars().map(fuzzy::fold).collect();
    let spans: Vec<(usize, usize)> = words
        .iter()
        .filter_map(|w| fuzzy::find(&w.chars, &folded, w.typos))
        .map(|Found { start, end, .. }| (start, end))
        .collect();
    Ok(Hit {
        id: scored.id,
        score: scored.score,
        snippet: fuzzy::snippet(&text, &spans),
    })
}
