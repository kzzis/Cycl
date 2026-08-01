//! ローカル完結の集中パターン分析。すべてSQLite上の実測データだけを使う。
//!
//! セッションの`started_at`はUTCで保存されているが、時間帯や月の集計は
//! 体感と一致させるためローカル時刻へ変換してから行う。

use crate::db::tag_queries;
use crate::error::AppResult;
use chrono::{DateTime, Datelike, Local, Timelike};
use rusqlite::Connection;
use shared::{calc_accuracy_score, AccuracyEntry, HourFocus, MonthlyStats, TagSummary};
use std::collections::HashMap;

const UNTAGGED: &str = "Untagged";
const UNTAGGED_COLOR: &str = "#8b8b9a";

fn to_local(ts: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Local))
}

/// `year_month`は "2026-07" 形式。ローカル時刻でその月に入るか判定する。
fn in_month(ts: &str, year_month: &str) -> bool {
    to_local(ts)
        .map(|dt| dt.format("%Y-%m").to_string() == year_month)
        .unwrap_or(false)
}

struct SessionRow {
    todo_id: i64,
    started_at: String,
    duration_secs: i64,
    completed: bool,
    interruptions: i64,
}

fn load_sessions(conn: &Connection) -> AppResult<Vec<SessionRow>> {
    let mut stmt = conn.prepare(
        "SELECT todo_id, started_at, duration_secs, completed, interruption_count
         FROM pomodoro_session WHERE duration_secs > 0",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(SessionRow {
                todo_id: r.get(0)?,
                started_at: r.get(1)?,
                duration_secs: r.get(2)?,
                completed: r.get::<_, i64>(3)? != 0,
                interruptions: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

struct EstimationRow {
    todo_id: i64,
    title: String,
    estimated: i64,
    actual: i64,
    score: f64,
    recorded_at: String,
}

fn load_estimations(conn: &Connection, todo_id: Option<i64>) -> AppResult<Vec<EstimationRow>> {
    let sql = "SELECT e.todo_id, t.title, e.estimated_count, e.actual_count,
                      e.accuracy_score, e.recorded_at
               FROM estimation_log e JOIN todo t ON t.id = e.todo_id";
    let map = |r: &rusqlite::Row| {
        Ok(EstimationRow {
            todo_id: r.get(0)?,
            title: r.get(1)?,
            estimated: r.get(2)?,
            actual: r.get(3)?,
            score: r.get(4)?,
            recorded_at: r.get(5)?,
        })
    };
    let mut rows = Vec::new();
    match todo_id {
        Some(id) => {
            let mut stmt = conn.prepare(&format!(
                "{sql} WHERE e.todo_id = ?1 ORDER BY e.recorded_at ASC"
            ))?;
            for row in stmt.query_map([id], map)? {
                rows.push(row?);
            }
        }
        None => {
            let mut stmt = conn.prepare(&format!("{sql} ORDER BY e.recorded_at ASC"))?;
            for row in stmt.query_map([], map)? {
                rows.push(row?);
            }
        }
    }
    Ok(rows)
}

/// タスク完了時に見積もりと実績の乖離を記録する。
/// 見積もり(target_count)が無いタスクは記録しない。
pub fn record_estimation(conn: &Connection, todo_id: i64) -> AppResult<()> {
    let row = conn.query_row(
        "SELECT target_count, pomodoro_count FROM todo WHERE id = ?1",
        [todo_id],
        |r| Ok((r.get::<_, Option<i64>>(0)?, r.get::<_, i64>(1)?)),
    );
    let Ok((Some(estimated), actual)) = row else {
        return Ok(());
    };
    let score = calc_accuracy_score(estimated, actual);
    conn.execute(
        "INSERT INTO estimation_log (todo_id, estimated_count, actual_count, accuracy_score)
         VALUES (?1, ?2, ?3, ?4)",
        (todo_id, estimated, actual, score),
    )?;
    Ok(())
}

/// 指定した月("2026-07")のサマリー。
pub fn monthly_stats(conn: &Connection, year_month: &str) -> AppResult<MonthlyStats> {
    let sessions = load_sessions(conn)?;
    let mut total_focus_secs = 0;
    let mut completed_sessions = 0;
    let mut interruptions = 0;
    for s in sessions
        .iter()
        .filter(|s| in_month(&s.started_at, year_month))
    {
        total_focus_secs += s.duration_secs;
        if s.completed {
            completed_sessions += 1;
            interruptions += s.interruptions;
        }
    }

    let scores: Vec<f64> = load_estimations(conn, None)?
        .into_iter()
        .filter(|e| in_month(&e.recorded_at, year_month))
        .map(|e| e.score)
        .collect();

    Ok(MonthlyStats {
        total_focus_secs,
        completed_sessions,
        avg_interruptions: if completed_sessions > 0 {
            interruptions as f64 / completed_sessions as f64
        } else {
            0.0
        },
        avg_accuracy: if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f64>() / scores.len() as f64
        },
    })
}

/// 予測精度ログ。`todo_id`を指定するとそのタスクの分だけ返す。
pub fn accuracy_entries(conn: &Connection, todo_id: Option<i64>) -> AppResult<Vec<AccuracyEntry>> {
    Ok(load_estimations(conn, todo_id)?
        .into_iter()
        .map(|e| AccuracyEntry {
            todo_title: e.title,
            estimated_count: e.estimated,
            actual_count: e.actual,
            accuracy_score: e.score,
            recorded_at: e.recorded_at,
        })
        .collect())
}

/// 指定した月の、曜日×時間帯ごとの集中時間(ヒートマップ用)。
pub fn focus_hours(conn: &Connection, year_month: &str) -> AppResult<Vec<HourFocus>> {
    let mut buckets: HashMap<(i64, i64), i64> = HashMap::new();
    for s in load_sessions(conn)? {
        let Some(dt) = to_local(&s.started_at) else {
            continue;
        };
        if dt.format("%Y-%m").to_string() != year_month {
            continue;
        }
        let weekday = dt.weekday().num_days_from_monday() as i64;
        let hour = dt.hour() as i64;
        *buckets.entry((weekday, hour)).or_insert(0) += s.duration_secs;
    }

    let mut result: Vec<HourFocus> = buckets
        .into_iter()
        .map(|((weekday, hour), secs)| HourFocus {
            weekday,
            hour,
            secs,
        })
        .collect();
    result.sort_by_key(|h| (h.weekday, h.hour));
    Ok(result)
}

/// 指定した月のタグ別サマリー(集中時間・平均実績ポモドーロ数・平均精度)。
/// 複数タグを持つタスクの時間は各タグへ均等按分し、タグ無しは "Untagged" に集約する。
pub fn tag_summary(conn: &Connection, year_month: &str) -> AppResult<Vec<TagSummary>> {
    // タグ名 -> (色, 秒, 実績ポモドーロ合計, 精度合計, 完了タスク数)
    let mut agg: HashMap<String, (String, f64, f64, f64, usize)> = HashMap::new();

    for s in load_sessions(conn)? {
        if !in_month(&s.started_at, year_month) {
            continue;
        }
        let tags = tag_queries::tags_for_todo(conn, s.todo_id)?;
        if tags.is_empty() {
            agg.entry(UNTAGGED.to_string())
                .or_insert_with(|| (UNTAGGED_COLOR.to_string(), 0.0, 0.0, 0.0, 0))
                .1 += s.duration_secs as f64;
        } else {
            let share = s.duration_secs as f64 / tags.len() as f64;
            for tag in tags {
                agg.entry(tag.name.clone())
                    .or_insert_with(|| (tag.color.clone(), 0.0, 0.0, 0.0, 0))
                    .1 += share;
            }
        }
    }

    for e in load_estimations(conn, None)? {
        if !in_month(&e.recorded_at, year_month) {
            continue;
        }
        let tags = tag_queries::tags_for_todo(conn, e.todo_id)?;
        let names: Vec<(String, String)> = if tags.is_empty() {
            vec![(UNTAGGED.to_string(), UNTAGGED_COLOR.to_string())]
        } else {
            tags.into_iter().map(|t| (t.name, t.color)).collect()
        };
        for (name, color) in names {
            let entry = agg.entry(name).or_insert_with(|| (color, 0.0, 0.0, 0.0, 0));
            entry.2 += e.actual as f64;
            entry.3 += e.score;
            entry.4 += 1;
        }
    }

    let mut result: Vec<TagSummary> = agg
        .into_iter()
        .map(|(name, (color, secs, pomos, scores, count))| TagSummary {
            name,
            color,
            secs: secs.round() as i64,
            avg_pomodoros: if count > 0 { pomos / count as f64 } else { 0.0 },
            avg_accuracy: if count > 0 {
                scores / count as f64
            } else {
                0.0
            },
        })
        .filter(|t| t.secs > 0 || t.avg_pomodoros > 0.0)
        .collect();
    result.sort_by_key(|t| std::cmp::Reverse(t.secs));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{migrations, session_queries, todo_queries};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    /// ローカル時刻で確実にその月に入るRFC3339文字列を作る(月末境界のズレを避ける)。
    fn local_ts(day: u32, hour: u32) -> String {
        use chrono::TimeZone;
        Local
            .with_ymd_and_hms(2026, 7, day, hour, 0, 0)
            .unwrap()
            .to_rfc3339()
    }

    fn month() -> String {
        "2026-07".to_string()
    }

    /// `record_estimation`は`recorded_at`に実時刻を入れるので、月で絞る
    /// テストではセッションと同じ月へ寄せておく(実行月に結果が左右されないように)。
    fn pin_estimations_to_test_month(conn: &Connection) {
        conn.execute(
            "UPDATE estimation_log SET recorded_at = ?1",
            [local_ts(10, 9)],
        )
        .unwrap();
    }

    #[test]
    fn record_estimation_skips_todos_without_a_target() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "no target", None).unwrap();
        record_estimation(&conn, todo.id).unwrap();
        assert!(accuracy_entries(&conn, None).unwrap().is_empty());
    }

    #[test]
    fn record_estimation_scores_estimate_against_actual() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "write docs", Some(3)).unwrap();
        todo_queries::increment_pomodoro_count(&conn, todo.id).unwrap();
        todo_queries::increment_pomodoro_count(&conn, todo.id).unwrap();

        record_estimation(&conn, todo.id).unwrap();

        let entries = accuracy_entries(&conn, None).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].estimated_count, 3);
        assert_eq!(entries[0].actual_count, 2);
        assert_eq!(entries[0].accuracy_score, calc_accuracy_score(3, 2));
        assert_eq!(entries[0].todo_title, "write docs");
    }

    #[test]
    fn monthly_stats_averages_interruptions_over_completed_sessions() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "work", None).unwrap();
        // 完了2本(中断3回と1回) + 途中経過1本。
        session_queries::record_focus(&conn, todo.id, &local_ts(10, 9), 1500, true, 3).unwrap();
        session_queries::record_focus(&conn, todo.id, &local_ts(11, 9), 1500, true, 1).unwrap();
        session_queries::record_focus(&conn, todo.id, &local_ts(12, 9), 300, false, 0).unwrap();

        let stats = monthly_stats(&conn, &month()).unwrap();
        assert_eq!(stats.total_focus_secs, 3300);
        assert_eq!(stats.completed_sessions, 2);
        assert_eq!(stats.avg_interruptions, 2.0);
    }

    #[test]
    fn monthly_stats_ignores_other_months() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "work", None).unwrap();
        session_queries::record_focus(&conn, todo.id, &local_ts(10, 9), 1500, true, 0).unwrap();

        let stats = monthly_stats(&conn, "2026-06").unwrap();
        assert_eq!(stats.total_focus_secs, 0);
        assert_eq!(stats.completed_sessions, 0);
    }

    #[test]
    fn focus_hours_buckets_by_weekday_and_hour() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "work", None).unwrap();
        // 2026-07-10 の 9時に2本 → 同じバケットへ合算。
        session_queries::record_focus(&conn, todo.id, &local_ts(10, 9), 600, true, 0).unwrap();
        session_queries::record_focus(&conn, todo.id, &local_ts(10, 9), 300, true, 0).unwrap();
        session_queries::record_focus(&conn, todo.id, &local_ts(10, 14), 120, true, 0).unwrap();

        let hours = focus_hours(&conn, &month()).unwrap();
        let at = |h: i64| hours.iter().find(|x| x.hour == h).map(|x| x.secs);
        assert_eq!(at(9), Some(900));
        assert_eq!(at(14), Some(120));
    }

    #[test]
    fn tag_summary_splits_time_and_averages_accuracy() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "tagged", Some(2)).unwrap();
        let work = tag_queries::create(&conn, "work", "#ff0000").unwrap();
        let deep = tag_queries::create(&conn, "deep", "#00ff00").unwrap();
        tag_queries::add_to_todo(&conn, todo.id, work.id).unwrap();
        tag_queries::add_to_todo(&conn, todo.id, deep.id).unwrap();

        session_queries::record_focus(&conn, todo.id, &local_ts(10, 9), 600, true, 0).unwrap();
        todo_queries::increment_pomodoro_count(&conn, todo.id).unwrap();
        todo_queries::increment_pomodoro_count(&conn, todo.id).unwrap();
        record_estimation(&conn, todo.id).unwrap();
        pin_estimations_to_test_month(&conn);

        let summary = tag_summary(&conn, &month()).unwrap();
        let get = |name: &str| summary.iter().find(|t| t.name == name).cloned().unwrap();
        // 600秒が2タグへ按分される。
        assert_eq!(get("work").secs, 300);
        assert_eq!(get("deep").secs, 300);
        // 見積もり2 = 実績2 なので精度は満点。
        assert_eq!(get("work").avg_accuracy, 1.0);
        assert_eq!(get("work").avg_pomodoros, 2.0);
    }
}
