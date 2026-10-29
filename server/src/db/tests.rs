use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;

use super::migrate::{self, MIGRATIONS};
use super::*;

/// A fresh, empty temp dir, removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "polykov-db-{name}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn versions(conn: &Connection) -> Vec<u32> {
    let mut stmt = conn
        .prepare("SELECT version FROM schema_version ORDER BY version")
        .unwrap();
    stmt.query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

#[test]
fn open_creates_file_in_wal_mode_with_foreign_keys() {
    let dir = TempDir::new("open");
    let db = Db::open(&dir.0).unwrap();
    assert!(dir.0.join(FILE_NAME).is_file());
    let conn = db.conn.lock().unwrap();
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    let fk: i64 = conn
        .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
    assert_eq!(fk, 1);
}

#[test]
fn fresh_db_migrates_to_latest() {
    let mut conn = Connection::open_in_memory().unwrap();
    assert_eq!(migrate::current(&conn).unwrap(), 0);
    migrate::run(&mut conn).unwrap();
    assert_eq!(migrate::current(&conn).unwrap(), migrate::latest());
    let expected: Vec<u32> = MIGRATIONS.iter().map(|(v, _)| *v).collect();
    assert_eq!(versions(&conn), expected);
}

#[test]
fn running_migrations_twice_is_a_no_op() {
    let dir = TempDir::new("twice");
    drop(Db::open(&dir.0).unwrap());
    let before = versions(&Connection::open(dir.0.join(FILE_NAME)).unwrap());
    drop(Db::open(&dir.0).unwrap());
    let after = versions(&Connection::open(dir.0.join(FILE_NAME)).unwrap());
    assert_eq!(before, after);
}

#[test]
fn pending_migrations_apply_once_in_order() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate::run(&mut conn).unwrap();
    let mut list = MIGRATIONS.to_vec();
    let next = migrate::latest() + 1;
    list.push((next, "CREATE TABLE a (x INTEGER);"));
    list.push((next + 1, "INSERT INTO a VALUES (1);"));
    migrate::run_list(&mut conn, &list).unwrap();
    migrate::run_list(&mut conn, &list).unwrap();
    let rows: i64 = conn
        .query_row("SELECT COUNT(*) FROM a", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rows, 1);
    assert_eq!(migrate::current(&conn).unwrap(), next + 1);
}

#[test]
fn failed_migration_is_rolled_back() {
    let mut conn = Connection::open_in_memory().unwrap();
    let mut list = MIGRATIONS.to_vec();
    let next = migrate::latest() + 1;
    list.push((next, "CREATE TABLE b (x INTEGER); NOT VALID SQL;"));
    let err = migrate::run_list(&mut conn, &list).unwrap_err();
    assert!(matches!(err, Error::Migration { version, .. } if version == next));
    assert_eq!(migrate::current(&conn).unwrap(), migrate::latest());
    let has_b: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = 'b'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(has_b, 0);
}

#[test]
fn newer_db_fails_open_with_clear_error() {
    let dir = TempDir::new("too-new");
    drop(Db::open(&dir.0).unwrap());
    let newer = migrate::latest() + 1;
    Connection::open(dir.0.join(FILE_NAME))
        .unwrap()
        .execute("INSERT INTO schema_version (version) VALUES (?1)", [newer])
        .unwrap();
    let err = Db::open(&dir.0).err().unwrap();
    assert!(matches!(err, Error::TooNew { db, binary } if db == newer && binary == newer - 1));
    assert_eq!(
        err.to_string(),
        format!(
            "database schema version {newer} is newer than this binary supports ({}); upgrade polykov-server",
            newer - 1
        )
    );
}

#[test]
fn migration_list_matches_migrations_dir() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut files: Vec<String> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    files.sort();
    assert_eq!(files.len(), MIGRATIONS.len());
    for (i, (file, (version, sql))) in files.iter().zip(MIGRATIONS).enumerate() {
        assert_eq!(fs::read_to_string(dir.join(file)).unwrap(), *sql, "{file}");
        let expected = u32::try_from(i + 1).unwrap();
        assert_eq!(
            *version, expected,
            "migration versions must be 1, 2, 3, ..."
        );
        assert!(
            file.starts_with(&format!("{expected:04}_")) && file.ends_with(".sql"),
            "{file} should be named {expected:04}_<name>.sql"
        );
    }
}

#[test]
fn first_migration_creates_only_schema_version() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrate::run_list(&mut conn, &MIGRATIONS[..1]).unwrap();
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE name NOT LIKE 'sqlite_%'")
        .unwrap();
    let names: Vec<String> = stmt
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(names, ["schema_version"]);
}

#[test]
fn fts5_is_available() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE VIRTUAL TABLE t USING fts5(body);")
        .unwrap();
}
