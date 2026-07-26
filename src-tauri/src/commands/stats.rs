use crate::db::{session_queries, AppState};
use crate::error::AppResult;
use chrono::{Duration, Utc};
use shared::TagFocus;
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
