use crate::db::{timing_queries, AppState};
use crate::error::AppResult;
use shared::Timing;
use tauri::State;

#[tauri::command]
pub fn timing_list(state: State<AppState>) -> AppResult<Vec<Timing>> {
    let conn = state.db.lock().unwrap();
    timing_queries::list(&conn)
}

#[tauri::command]
pub fn timing_create(state: State<AppState>, name: String, color: String) -> AppResult<Timing> {
    let conn = state.db.lock().unwrap();
    timing_queries::create(&conn, &name, &color)
}

#[tauri::command]
pub fn timing_delete(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    timing_queries::delete(&conn, id)
}
