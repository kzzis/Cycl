// Phase 3のタイマーエンジンから呼ばれるまではTauriコマンドとして未公開・未使用
#![allow(dead_code)]

use crate::db::tag_queries;
use crate::error::AppResult;
use crate::models::session::PomodoroSession;
use rusqlite::Connection;
use shared::TagFocus;
use std::collections::HashMap;

const UNTAGGED_COLOR: &str = "#8b8b9a";

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

pub fn record_completed(
    conn: &Connection,
    todo_id: i64,
    started_at: &str,
) -> AppResult<PomodoroSession> {
    conn.execute(
        "INSERT INTO pomodoro_session (todo_id, started_at, completed) VALUES (?1, ?2, 1)",
        (todo_id, started_at),
    )?;
    get(conn, conn.last_insert_rowid())
}

/// 作業チャンク(一時停止・切替・完了時の経過)を記録する。統計グラフの元データ。
/// `interruptions`はフェーズを完走した行にだけ入る(途中経過の行は0)。
pub fn record_focus(
    conn: &Connection,
    todo_id: i64,
    at: &str,
    duration_secs: i64,
    completed: bool,
    interruptions: i64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO pomodoro_session
            (todo_id, started_at, duration_secs, completed, interruption_count)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (todo_id, at, duration_secs, completed as i64, interruptions),
    )?;
    Ok(())
}

/// `cutoff`(RFC3339)以降の作業時間をタグ別に集計する。
/// 複数タグを持つタスクの時間は各タグへ均等按分し、タグ無しは "Untagged" に入れる。
pub fn focus_by_tag(conn: &Connection, cutoff: &str) -> AppResult<Vec<TagFocus>> {
    let mut stmt = conn.prepare(
        "SELECT todo_id, duration_secs FROM pomodoro_session
         WHERE started_at >= ?1 AND duration_secs > 0",
    )?;
    let rows = stmt
        .query_map([cutoff], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // 端数を丸めないよう f64 で集計し、最後に丸める。
    let mut agg: HashMap<String, (String, f64)> = HashMap::new();
    for (todo_id, dur) in rows {
        let tags = tag_queries::tags_for_todo(conn, todo_id)?;
        if tags.is_empty() {
            agg.entry("Untagged".to_string())
                .or_insert_with(|| (UNTAGGED_COLOR.to_string(), 0.0))
                .1 += dur as f64;
        } else {
            let share = dur as f64 / tags.len() as f64;
            for tag in tags {
                agg.entry(tag.name.clone())
                    .or_insert_with(|| (tag.color.clone(), 0.0))
                    .1 += share;
            }
        }
    }

    let mut result: Vec<TagFocus> = agg
        .into_iter()
        .map(|(name, (color, secs))| TagFocus {
            name,
            color,
            secs: secs.round() as i64,
        })
        .filter(|t| t.secs > 0)
        .collect();
    result.sort_by_key(|t| std::cmp::Reverse(t.secs));
    Ok(result)
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

    #[test]
    fn record_completed_inserts_an_already_completed_session() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "作業", None).unwrap();
        let session = record_completed(&conn, todo.id, "2026-07-04T10:00:00+09:00").unwrap();
        assert!(session.completed);
    }

    #[test]
    fn focus_by_tag_splits_multi_tag_time_and_buckets_untagged() {
        use crate::db::tag_queries;
        let conn = setup_conn();
        let a = todo_queries::create(&conn, "A", None).unwrap();
        let b = todo_queries::create(&conn, "B", None).unwrap();
        let work = tag_queries::create(&conn, "work", "#ff0000").unwrap();
        let home = tag_queries::create(&conn, "home", "#00ff00").unwrap();

        // A は work と home の2タグ → 600秒を300ずつ按分。
        tag_queries::add_to_todo(&conn, a.id, work.id).unwrap();
        tag_queries::add_to_todo(&conn, a.id, home.id).unwrap();
        record_focus(&conn, a.id, "2026-07-20T10:00:00+00:00", 600, true, 0).unwrap();
        // B はタグ無し → Untagged に 120秒。
        record_focus(&conn, b.id, "2026-07-20T11:00:00+00:00", 120, false, 0).unwrap();
        // cutoff より前は集計されない。
        record_focus(&conn, b.id, "2020-01-01T00:00:00+00:00", 999, false, 0).unwrap();

        let result = focus_by_tag(&conn, "2026-07-01T00:00:00+00:00").unwrap();
        let get = |name: &str| result.iter().find(|t| t.name == name).map(|t| t.secs);
        assert_eq!(get("work"), Some(300));
        assert_eq!(get("home"), Some(300));
        assert_eq!(get("Untagged"), Some(120));
    }
}
