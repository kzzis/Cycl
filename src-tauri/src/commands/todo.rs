use crate::db::{todo_queries, AppState};
use crate::error::AppResult;
use crate::timer::engine::TimerEngine;
use shared::Todo;
use tauri::{AppHandle, Emitter, State};

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
pub fn todo_set_active(
    app: AppHandle,
    state: State<AppState>,
    engine: State<TimerEngine>,
    id: Option<i64>,
) -> AppResult<()> {
    // 切り替え前に、それまでの取り組み中タスクへ作業経過を記録する。
    engine.flush_focus();
    {
        let conn = state.db.lock().unwrap();
        todo_queries::set_active(&conn, id)?;
    }
    // タイマー画面など他のビューが取り組み中タスクの変更に追従できるよう通知する。
    let _ = app.emit("todos:changed", ());
    Ok(())
}

#[tauri::command]
pub fn todo_reorder(state: State<AppState>, ordered_ids: Vec<i64>) -> AppResult<Vec<Todo>> {
    let conn = state.db.lock().unwrap();
    todo_queries::reorder(&conn, &ordered_ids)?;
    todo_queries::list(&conn)
}

#[tauri::command]
pub fn todo_list_by_category(state: State<AppState>, category: String) -> AppResult<Vec<Todo>> {
    let conn = state.db.lock().unwrap();
    todo_queries::list_by_category(&conn, &category)
}

#[tauri::command]
pub fn todo_update_category(state: State<AppState>, id: i64, category: String) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    todo_queries::update_category(&conn, id, &category)
}
