use crate::error::AppResult;
use rusqlite::Connection;
use shared::{Timing, DEFAULT_TIMING};

fn timing_from_row(row: &rusqlite::Row) -> rusqlite::Result<Timing> {
    Ok(Timing {
        id: row.get("id")?,
        key: row.get("key")?,
        name: row.get("name")?,
        color: row.get("color")?,
        is_builtin: row.get::<_, i64>("is_builtin")? != 0,
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<Timing>> {
    let mut stmt = conn
        .prepare("SELECT id, key, name, color, is_builtin FROM timing ORDER BY sort_order ASC")?;
    let timings = stmt
        .query_map([], timing_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(timings)
}

/// カスタムタイミングを追加する。`key`は名前をそのまま使う(UNIQUE制約で重複を弾く)。
pub fn create(conn: &Connection, name: &str, color: &str) -> AppResult<Timing> {
    conn.execute(
        "INSERT INTO timing (key, name, color, sort_order, is_builtin)
         VALUES (?1, ?2, ?3, (SELECT COALESCE(MAX(sort_order), -1) FROM timing) + 1, 0)",
        (name, name, color),
    )?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row(
        "SELECT id, key, name, color, is_builtin FROM timing WHERE id = ?1",
        [id],
        timing_from_row,
    )?)
}

/// カスタムタイミングを削除する。組み込みは削除しない。
/// 削除するタイミングに属していたTodoは既定タイミングへ戻す。
pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let row = conn.query_row(
        "SELECT key, is_builtin FROM timing WHERE id = ?1",
        [id],
        |r| {
            Ok((
                r.get::<_, String>("key")?,
                r.get::<_, i64>("is_builtin")? != 0,
            ))
        },
    );
    let (key, is_builtin) = match row {
        Ok(v) => v,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    if is_builtin {
        return Ok(());
    }
    conn.execute(
        "UPDATE todo SET category = ?1 WHERE category = ?2",
        (DEFAULT_TIMING, &key),
    )?;
    conn.execute("DELETE FROM timing WHERE id = ?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{migrations, todo_queries};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn list_returns_seeded_builtins() {
        let conn = setup_conn();
        let timings = list(&conn).unwrap();
        assert_eq!(timings.len(), 6);
        assert_eq!(timings[0].key, "today");
        assert!(timings.iter().all(|t| t.is_builtin));
    }

    #[test]
    fn create_adds_a_custom_timing() {
        let conn = setup_conn();
        let timing = create(&conn, "Weekend", "#ff0000").unwrap();
        assert_eq!(timing.key, "Weekend");
        assert!(!timing.is_builtin);
        assert_eq!(list(&conn).unwrap().len(), 7);
    }

    #[test]
    fn deleting_custom_timing_reassigns_todos_to_default() {
        let conn = setup_conn();
        let timing = create(&conn, "Weekend", "#ff0000").unwrap();
        let todo = todo_queries::create(&conn, "作業", None).unwrap();
        todo_queries::update_category(&conn, todo.id, &timing.key).unwrap();

        delete(&conn, timing.id).unwrap();

        assert_eq!(list(&conn).unwrap().len(), 6);
        let moved = todo_queries::get(&conn, todo.id).unwrap();
        assert_eq!(moved.category, DEFAULT_TIMING);
    }

    #[test]
    fn builtin_timing_is_not_deleted() {
        let conn = setup_conn();
        let today = list(&conn)
            .unwrap()
            .into_iter()
            .find(|t| t.key == "today")
            .unwrap();
        delete(&conn, today.id).unwrap();
        assert_eq!(list(&conn).unwrap().len(), 6);
    }
}
