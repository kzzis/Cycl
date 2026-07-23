use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use shared::Todo;

const SELECT_COLUMNS: &str =
    "id, title, is_completed, pomodoro_count, target_count, is_active, created_at";

fn todo_from_row(row: &rusqlite::Row) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: row.get("id")?,
        title: row.get("title")?,
        is_completed: row.get::<_, i64>("is_completed")? != 0,
        pomodoro_count: row.get("pomodoro_count")?,
        target_count: row.get("target_count")?,
        is_active: row.get::<_, i64>("is_active")? != 0,
        created_at: row.get("created_at")?,
    })
}

pub fn list(conn: &Connection) -> AppResult<Vec<Todo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM todo ORDER BY sort_order ASC"
    ))?;
    let todos = stmt
        .query_map([], todo_from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(todos)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Todo> {
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM todo WHERE id = ?1"),
        [id],
        todo_from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::TodoNotFound(id),
        other => AppError::Database(other),
    })
}

pub fn create(conn: &Connection, title: &str, target_count: Option<i64>) -> AppResult<Todo> {
    conn.execute(
        "INSERT INTO todo (title, target_count, sort_order)
         VALUES (?1, ?2, (SELECT COALESCE(MAX(sort_order), -1) FROM todo) + 1)",
        (title, target_count),
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn update(
    conn: &Connection,
    id: i64,
    title: &str,
    target_count: Option<i64>,
) -> AppResult<Todo> {
    let affected = conn.execute(
        "UPDATE todo SET title = ?1, target_count = ?2 WHERE id = ?3",
        (title, target_count, id),
    )?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let affected = conn.execute("DELETE FROM todo WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    Ok(())
}

pub fn toggle_complete(conn: &Connection, id: i64) -> AppResult<Todo> {
    let affected = conn.execute(
        "UPDATE todo SET is_completed = NOT is_completed WHERE id = ?1",
        [id],
    )?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    get(conn, id)
}

/// 「現在取り組むTodo」を切り替える。常に高々1件だけがactiveになるよう、
/// 一度全解除してから指定されたidだけ立てる。
pub fn set_active(conn: &Connection, id: Option<i64>) -> AppResult<()> {
    conn.execute("UPDATE todo SET is_active = 0", [])?;
    if let Some(id) = id {
        let affected = conn.execute("UPDATE todo SET is_active = 1 WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(AppError::TodoNotFound(id));
        }
    }
    Ok(())
}

/// 「現在取り組むTodo」を取得する。存在しなければ`None`。
pub fn get_active(conn: &Connection) -> AppResult<Option<Todo>> {
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM todo WHERE is_active = 1"),
        [],
        todo_from_row,
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(AppError::Database(other)),
    })
}

/// ドラッグ&ドロップで確定した新しい順序を反映する。
/// `ordered_ids`の並び順そのままを`sort_order`(0始まりの連番)として書き込む。
pub fn reorder(conn: &Connection, ordered_ids: &[i64]) -> AppResult<()> {
    let tx = conn.unchecked_transaction()?;
    for (index, id) in ordered_ids.iter().enumerate() {
        tx.execute(
            "UPDATE todo SET sort_order = ?1 WHERE id = ?2",
            (index as i64, id),
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// ポモドーロセッション完了時にタイマーエンジンから呼ばれる。
pub fn increment_pomodoro_count(conn: &Connection, id: i64) -> AppResult<Todo> {
    let affected = conn.execute(
        "UPDATE todo SET pomodoro_count = pomodoro_count + 1 WHERE id = ?1",
        [id],
    )?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    get(conn, id)
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
    fn create_and_get_todo() {
        let conn = setup_conn();
        let todo = create(&conn, "牛乳を買う", Some(4)).unwrap();
        assert_eq!(todo.title, "牛乳を買う");
        assert_eq!(todo.target_count, Some(4));
        assert!(!todo.is_completed);

        let fetched = get(&conn, todo.id).unwrap();
        assert_eq!(fetched.id, todo.id);
    }

    #[test]
    fn get_missing_todo_returns_not_found() {
        let conn = setup_conn();
        let err = get(&conn, 999).unwrap_err();
        assert!(matches!(err, AppError::TodoNotFound(999)));
    }

    #[test]
    fn toggle_complete_flips_state() {
        let conn = setup_conn();
        let todo = create(&conn, "掃除", None).unwrap();
        let toggled = toggle_complete(&conn, todo.id).unwrap();
        assert!(toggled.is_completed);
        let toggled_again = toggle_complete(&conn, todo.id).unwrap();
        assert!(!toggled_again.is_completed);
    }

    #[test]
    fn set_active_only_allows_one_at_a_time() {
        let conn = setup_conn();
        let a = create(&conn, "A", None).unwrap();
        let b = create(&conn, "B", None).unwrap();

        set_active(&conn, Some(a.id)).unwrap();
        set_active(&conn, Some(b.id)).unwrap();

        let a_after = get(&conn, a.id).unwrap();
        let b_after = get(&conn, b.id).unwrap();
        assert!(!a_after.is_active);
        assert!(b_after.is_active);
    }

    #[test]
    fn increment_pomodoro_count_increases_by_one() {
        let conn = setup_conn();
        let todo = create(&conn, "作業", None).unwrap();
        let updated = increment_pomodoro_count(&conn, todo.id).unwrap();
        assert_eq!(updated.pomodoro_count, 1);
    }

    #[test]
    fn create_appends_new_todo_after_existing_ones() {
        let conn = setup_conn();
        let a = create(&conn, "A", None).unwrap();
        let b = create(&conn, "B", None).unwrap();

        let listed = list(&conn).unwrap();
        assert_eq!(
            listed.iter().map(|t| t.id).collect::<Vec<_>>(),
            [a.id, b.id]
        );
    }

    #[test]
    fn reorder_changes_the_list_order() {
        let conn = setup_conn();
        let a = create(&conn, "A", None).unwrap();
        let b = create(&conn, "B", None).unwrap();
        let c = create(&conn, "C", None).unwrap();

        reorder(&conn, &[c.id, a.id, b.id]).unwrap();

        let listed = list(&conn).unwrap();
        assert_eq!(
            listed.iter().map(|t| t.id).collect::<Vec<_>>(),
            [c.id, a.id, b.id]
        );
    }

    #[test]
    fn get_active_returns_the_active_todo() {
        let conn = setup_conn();
        let a = create(&conn, "A", None).unwrap();
        let _b = create(&conn, "B", None).unwrap();

        assert!(get_active(&conn).unwrap().is_none());

        set_active(&conn, Some(a.id)).unwrap();
        let active = get_active(&conn).unwrap().unwrap();
        assert_eq!(active.id, a.id);
    }
}
