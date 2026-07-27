use crate::db::{analytics_queries, session_queries, AppState};
use crate::error::AppResult;
use chrono::{Duration, Utc};
use shared::{AccuracyEntry, HourFocus, MonthlyStats, TagFocus, TagSummary};
use tauri::State;

/// 直近の作業時間をタグ別に集計する。period は "week" | "month" | "year"
/// (それぞれ直近7日 / 30日 / 365日)。
#[tauri::command]
pub fn stats_focus_by_tag(state: State<AppState>, period: String) -> AppResult<Vec<TagFocus>> {
    let days = match period.as_str() {
        "month" => 30,
        "year" => 365,
        _ => 7,
    };
    let cutoff = (Utc::now() - Duration::days(days)).to_rfc3339();
    let conn = state.db.lock().unwrap();
    session_queries::focus_by_tag(&conn, &cutoff)
}

/// 月次サマリー。`year_month`は "2026-07" 形式。
#[tauri::command]
pub fn stats_monthly(state: State<AppState>, year_month: String) -> AppResult<MonthlyStats> {
    let conn = state.db.lock().unwrap();
    analytics_queries::monthly_stats(&conn, &year_month)
}

/// 予測精度ログ。`todo_id`省略時は全タスク分を古い順に返す。
#[tauri::command]
pub fn stats_accuracy(
    state: State<AppState>,
    todo_id: Option<i64>,
) -> AppResult<Vec<AccuracyEntry>> {
    let conn = state.db.lock().unwrap();
    analytics_queries::accuracy_entries(&conn, todo_id)
}

/// 曜日×時間帯の集中度。
#[tauri::command]
pub fn stats_focus_hours(state: State<AppState>, year_month: String) -> AppResult<Vec<HourFocus>> {
    let conn = state.db.lock().unwrap();
    analytics_queries::focus_hours(&conn, &year_month)
}

/// タグ別の月次サマリー。
#[tauri::command]
pub fn stats_tag_summary(state: State<AppState>, year_month: String) -> AppResult<Vec<TagSummary>> {
    let conn = state.db.lock().unwrap();
    analytics_queries::tag_summary(&conn, &year_month)
}
