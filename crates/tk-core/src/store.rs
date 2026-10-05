use rusqlite::Connection;
use std::path::Path;
use crate::error::Result;

pub struct Store{
    conn:rusqlite::Connection,
}

const MIGRATIONS:&[&str]=&[
    "CREATE TABLE notes(
        id INTEGER PRIMARY KEY,
        body TEXT NOT NULL,
        created_at TEXT NOT NULL
    )"

];

impl Store{

    fn from_connection(mut conn:Connection)->Result<Store>{
    migrate(&mut conn)?;
    Ok(Store { conn })
    }

    pub fn open(path:impl AsRef<Path>)->Result<Store>{
        let conn=Connection::open(path)?;
        Self::from_connection(conn)
    }

    pub fn open_in_memory()->Result<Store>{
        let conn=Connection::open_in_memory()?;
        Self::from_connection(conn)
    }
}

fn migrate(conn:&mut Connection)->Result<()>{
    let current: i64=conn.query_row(
        "PRAGMA user_version",
        [], 
        |row| row.get(0)
    )?;
    for (i,sql) in MIGRATIONS.iter().enumerate(){
        if (i as i64) < current{
            continue;
        }
        let tx=conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", (i+1) as i64)?;
        tx.commit()?;
    };

    Ok(())
}

#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn db_version_test(){
        let store=Store::open_in_memory().unwrap();
        let user_version:i64=store.conn.query_row(
            "PRAGMA user_version", [], |row|row.get(0)).unwrap();
        assert_eq!(user_version,MIGRATIONS.len() as i64)

    }
    fn reopen_keeps_data(){
        let dir=tempfile::tempdir().unwrap();
        let path=dir.path().join("test.db");

        {
            let store=Store::open(&path);
            store
            .unwrap().conn
            .execute(
                "INSERT INTO notes (body, created_at) VALUES (?1, ?2)",
                ["우유 사기", "2026-01-01T00:00:00Z"],
            )
            .unwrap();
        }
        let store=Store::open(&path).unwrap();
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
}
