use chrono::Utc;
use rusqlite::{params, Connection, Row};
use std::path::Path;

use crate::error::Error;
use crate::error::Result;
use crate::note::Note;

pub struct Store {
    conn: rusqlite::Connection,
}

const MIGRATIONS: &[&str] = &["CREATE TABLE notes(
        id INTEGER PRIMARY KEY,
        body TEXT NOT NULL,
        created_at TEXT NOT NULL
    )"];

//list_notes,search_notes 함수에서 필요한 DB 각 행들을 Note로 바꾸는 함수.
fn row_to_notes(row: &Row) -> rusqlite::Result<Note> {
    let id = row.get(0)?;
    let body = row.get(1)?;
    let created_at = row.get(2)?;

    Ok(Note {
        id,
        body,
        created_at,
    })
}
fn escape_like(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if matches!(c, '\\' | '%' | '_') {
            result.push('\\');
        }
        result.push(c);
    }
    result
}

impl Store {
    fn from_connection(mut conn: Connection) -> Result<Store> {
        migrate(&mut conn)?;
        Ok(Store { conn })
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Store> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory() -> Result<Store> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    //db func
    pub fn add_note(&self, body: &str) -> Result<Note> {
        let body = body.trim();
        if body.is_empty() {
            return Err(Error::EmptyNote);
        };
        let created_at = Utc::now();
        self.conn.execute(
            "INSERT INTO notes (body,created_at) VALUES (?1,?2)",
            params![body, created_at],
        )?;
        let id = self.conn.last_insert_rowid();
        let note = Note {
            id,
            body: body.to_string(),
            created_at,
        };
        Ok(note)
    }
    pub fn list_notes(&self, limit: usize) -> Result<Vec<Note>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id,body,created_at FROM notes ORDER BY id DESC LIMIT ?1")?;
        let note = stmt.query_map(params![limit as i64], row_to_notes)?;
        let notes = note.collect::<rusqlite::Result<Vec<Note>>>()?;

        Ok(notes)
    }
    pub fn search_notes(&self, keyword: &str) -> Result<Vec<Note>> {
        let mut stmt=self.conn.prepare(
            "SELECT id, body, created_at FROM notes WHERE body LIKE ?1 ESCAPE '\\' ORDER BY id DESC")?;
        let pattern = format!("%{}%", escape_like(keyword));
        let note = stmt.query_map(params![pattern], row_to_notes)?;
        let notes = note.collect::<rusqlite::Result<Vec<Note>>>()?;

        Ok(notes)
    }
    pub fn delete_note(&self, id: i64) -> Result<bool> {
        let rows = self.conn.execute("DELETE FROM notes WHERE id=?1", [id])?;
        Ok(rows > 0)
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate() {
        if (i as i64) < current {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i + 1) as i64)?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_store() -> Store {
        Store::open_in_memory().unwrap()
    }

    #[test]
    fn db_version_test() {
        let store = Store::open_in_memory().unwrap();
        let user_version: i64 = store
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(user_version, MIGRATIONS.len() as i64)
    }
    #[test]
    fn reopen_keeps_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");

        {
            let store = Store::open(&path);
            store
                .unwrap()
                .conn
                .execute(
                    "INSERT INTO notes (body, created_at) VALUES (?1, ?2)",
                    ["우유 사기", "2026-01-01T00:00:00Z"],
                )
                .unwrap();
        }
        let store = Store::open(&path).unwrap();
        let count: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let version: i64 = store
            .conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }

    //add_note
    #[test]
    fn add_note_trims_whitespace_test() {
        let store = new_store();
        let note = store.add_note("  apple   ").unwrap();
        assert_eq!(note.body, "apple");
    }
    #[test]
    fn add_note_returns_saved_note() {
        let store = new_store();
        let note = store.add_note("우유 사기").unwrap();
        assert_eq!(note.body, "우유 사기");
        assert_eq!(note.id, 1);
    }
    #[test]
    fn add_note_rejects_empty() {
        let store = new_store();
        let note = store.add_note("");
        assert!(matches!(note, Err(Error::EmptyNote)));
    }
    #[test]
    fn add_note_rejects_whitespace_only() {
        let store = new_store();
        let note = store.add_note("      ");
        assert!(matches!(note, Err(Error::EmptyNote)));
    }
    #[test]
    fn add_note_assigns_increasing_ids() {
        let store = new_store();
        let note = store.add_note("우유 사기").unwrap();
        assert_eq!(note.body, "우유 사기");
        assert_eq!(note.id, 1);
        let note = store.add_note("과일 사기").unwrap();
        assert_eq!(note.body, "과일 사기");
        assert_eq!(note.id, 2);
    }

    //list_notes()
    #[test]
    fn list_notes_empty_db() {
        let store = new_store();
        let _ = store.add_note("");
        assert!(store.list_notes(10).unwrap().is_empty())
    }
    #[test]
    fn list_notes_returns_added_notes() {
        let store = new_store();
        let note = store.add_note("우유 사기").unwrap();
        let list_note = store.list_notes(10).unwrap();
        assert_eq!(list_note.len(), 1);
        assert_eq!(list_note[0].body, "우유 사기");
    }
    #[test]
    fn list_notes_newest_first() {
        let store = new_store();
        let _ = store.add_note("first").unwrap();
        let _ = store.add_note("second").unwrap();
        let _ = store.add_note("third").unwrap();
        let list_note = store.list_notes(10).unwrap();
        assert_eq!(list_note[0].body, "third");
        assert_eq!(list_note[1].body, "second");
        assert_eq!(list_note[2].body, "first");
    }
    #[test]
    fn list_notes_respects_limit() {
        let store = new_store();
        let _ = store.add_note("first").unwrap();
        let _ = store.add_note("second").unwrap();
        let _ = store.add_note("third").unwrap();
        let list_note = store.list_notes(2).unwrap();
        assert_eq!(list_note.len(), 2);
        assert_eq!(list_note[0].body, "third");
        assert_eq!(list_note[1].body, "second");
    }
    //search note

    //delete note
    #[test]
    fn delete_note_removes_existing() {
        let store = new_store();
        let note = store.add_note("  apple ").unwrap();

        let id = note.id;
        let check = store.delete_note(id).unwrap();
        let list = store.list_notes(10).unwrap();
        assert!(check);
        assert!(list.is_empty());
    }
    #[test]
    fn delete_note_missing_returns_false() {
        let store = new_store();
        let note = store.add_note("  apple ").unwrap();
        let id = 100;
        let check = store.delete_note(id).unwrap();
        assert!(!check);
    }
    #[test]
    fn delete_note_twice() {
        let store = new_store();
        let note = store.add_note("  apple ").unwrap();

        let id = 1;
        let check = store.delete_note(id).unwrap();
        assert!(check);
        let check_2 = store.delete_note(id).unwrap();
        assert!(!check_2);
    }
    #[test]
    fn delete_note_keeps_others() {
        let store = new_store();
        let _ = store.add_note("first");
        let _ = store.add_note("second");
        let _ = store.add_note("third");

        let id = 2;
        let _ = store.delete_note(id);

        let list_note = store.list_notes(10).unwrap();
        assert_eq!(list_note.len(), 2);
        assert_eq!(list_note[0].body, "third");
        assert_eq!(list_note[1].body, "first");
    }

    //search_notes
    #[test]
    fn search_finds_matching() {
        let store = new_store();
        store.add_note("우유 사기").unwrap();
        store.add_note("세금 신고").unwrap();

        let result = store.search_notes("우유").unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body, "우유 사기");
    }

    #[test]
    fn search_no_match_returns_empty() {
        let store = new_store();
        store.add_note("우유 사기").unwrap();

        let result = store.search_notes("커피").unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn search_matches_substring() {
        let store = new_store();
        store.add_note("세금 신고").unwrap();

        let result = store.search_notes("금 신").unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body, "세금 신고");
    }

    #[test]
    fn search_newest_first() {
        let store = new_store();
        store.add_note("우유 1").unwrap();
        store.add_note("우유 2").unwrap();

        let result = store.search_notes("우유").unwrap();
        let bodies: Vec<&str> = result.iter().map(|n| n.body.as_str()).collect();

        assert_eq!(bodies, vec!["우유 2", "우유 1"]);
    }

    #[test]
    fn search_treats_percent_literally() {
        let store = new_store();
        store.add_note("100% 확인").unwrap();
        store.add_note("1000원 환전").unwrap();

        let result = store.search_notes("100%").unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body, "100% 확인");
    }

    #[test]
    fn search_treats_underscore_literally() {
        let store = new_store();
        store.add_note("a_b").unwrap();
        store.add_note("axb").unwrap();

        let result = store.search_notes("a_b").unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].body, "a_b");
    }
    #[test]
    fn escape_like_test() {
        let s = "abc_%\\";
        assert_eq!(escape_like(s), "abc\\_\\%\\\\".to_string())
    }
}
