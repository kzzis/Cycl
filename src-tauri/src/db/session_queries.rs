use crate::error::AppResult;
use crate::models::session::PomodoroSession;
use rusqlite::Connection;

pub fn create(conn: &Connection, todo_id: i64, started_at: &str) -> AppResult<PomodoroSession> {
    conn.execute(
        "INSERT INTO pomodoro_session (todo_id, started_at) VALUES (?1, ?2)",
        (todo_id, started_at),
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn complete(conn: &Connection, id: i64) -> AppResult<PomodoroSession> {
    conn.execute(
        "UPDATE pomodoro_session SET completed = 1 WHERE id = ?1",
        [id],
    )?;
    get(conn, id)
}

pub fn list_by_todo(conn: &Connection, todo_id: i64) -> AppResult<Vec<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, todo_id, started_at, completed FROM pomodoro_session
         WHERE todo_id = ?1 ORDER BY started_at ASC",
    )?;
    let sessions = stmt
        .query_map([todo_id], PomodoroSession::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sessions)
}

fn get(conn: &Connection, id: i64) -> AppResult<PomodoroSession> {
    Ok(conn.query_row(
        "SELECT id, todo_id, started_at, completed FROM pomodoro_session WHERE id = ?1",
        [id],
        PomodoroSession::from_row,
    )?)
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
    fn create_and_complete_session() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "作業", None).unwrap();

        let session = create(&conn, todo.id, "2026-07-04T10:00:00.000Z").unwrap();
        assert!(!session.completed);

        let completed = complete(&conn, session.id).unwrap();
        assert!(completed.completed);
    }

    #[test]
    fn list_by_todo_returns_only_matching_sessions() {
        let conn = setup_conn();
        let todo_a = todo_queries::create(&conn, "A", None).unwrap();
        let todo_b = todo_queries::create(&conn, "B", None).unwrap();

        create(&conn, todo_a.id, "2026-07-04T10:00:00.000Z").unwrap();
        create(&conn, todo_a.id, "2026-07-04T10:30:00.000Z").unwrap();
        create(&conn, todo_b.id, "2026-07-04T11:00:00.000Z").unwrap();

        let sessions = list_by_todo(&conn, todo_a.id).unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
