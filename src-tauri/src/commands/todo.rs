use crate::db::{todo_queries, AppState};
use crate::error::AppResult;
use crate::models::todo::Todo;
use tauri::State;

#[tauri::command]
pub fn todo_list(state: State<AppState>) -> AppResult<Vec<Todo>> {
    let conn = state.db.lock().unwrap();
    todo_queries::list(&conn)
}

#[tauri::command]
pub fn todo_create(
    state: State<AppState>,
    title: String,
    target_count: Option<i64>,
) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    todo_queries::create(&conn, &title, target_count)
}

#[tauri::command]
pub fn todo_update(
    state: State<AppState>,
    id: i64,
    title: String,
    target_count: Option<i64>,
) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    todo_queries::update(&conn, id, &title, target_count)
}

#[tauri::command]
pub fn todo_delete(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    todo_queries::delete(&conn, id)
}

#[tauri::command]
pub fn todo_toggle_complete(state: State<AppState>, id: i64) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    todo_queries::toggle_complete(&conn, id)
}

#[tauri::command]
pub fn todo_set_active(state: State<AppState>, id: Option<i64>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    todo_queries::set_active(&conn, id)
}
