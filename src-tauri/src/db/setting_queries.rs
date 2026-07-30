//! アプリ設定の汎用key-valueストア。

use crate::error::AppResult;
use rusqlite::Connection;

pub fn get(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let value = conn
        .query_row("SELECT value FROM app_setting WHERE key = ?1", [key], |r| {
            r.get::<_, String>(0)
        })
        .ok();
    Ok(value)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_setting (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        (key, value),
    )?;
    Ok(())
}

/// 真偽値として読む。未設定なら`default`。
pub fn get_bool(conn: &Connection, key: &str, default: bool) -> AppResult<bool> {
    Ok(get(conn, key)?.map(|v| v == "1").unwrap_or(default))
}

pub fn set_bool(conn: &Connection, key: &str, value: bool) -> AppResult<()> {
    set(conn, key, if value { "1" } else { "0" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn missing_key_falls_back_to_default() {
        let conn = setup_conn();
        assert!(get(&conn, "nope").unwrap().is_none());
        assert!(get_bool(&conn, "nope", true).unwrap());
        assert!(!get_bool(&conn, "nope", false).unwrap());
    }

    #[test]
    fn set_overwrites_an_existing_key() {
        let conn = setup_conn();
        set_bool(&conn, "mcp_enabled", true).unwrap();
        assert!(get_bool(&conn, "mcp_enabled", false).unwrap());
        set_bool(&conn, "mcp_enabled", false).unwrap();
        assert!(!get_bool(&conn, "mcp_enabled", true).unwrap());
    }
}
