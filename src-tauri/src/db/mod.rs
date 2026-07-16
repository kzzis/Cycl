use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod migrations;
pub mod session_queries;
pub mod todo_queries;

pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}

pub fn open(db_path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    migrations::run(&conn)?;
    Ok(conn)
}
