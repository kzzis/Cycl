use crate::db::{tag_queries, todo_queries, AppState};
use crate::error::AppResult;
use shared::{Tag, Todo};
use tauri::State;

#[tauri::command]
pub fn tag_list(state: State<AppState>) -> AppResult<Vec<Tag>> {
    let conn = state.db.lock().unwrap();
    tag_queries::list(&conn)
}

#[tauri::command]
pub fn tag_create(state: State<AppState>, name: String, color: String) -> AppResult<Tag> {
    let conn = state.db.lock().unwrap();
    tag_queries::create(&conn, &name, &color)
}

#[tauri::command]
pub fn tag_delete(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    tag_queries::delete(&conn, id)
}

#[tauri::command]
pub fn todo_add_tag(state: State<AppState>, todo_id: i64, tag_id: i64) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    tag_queries::add_to_todo(&conn, todo_id, tag_id)?;
    todo_queries::get(&conn, todo_id)
}

#[tauri::command]
pub fn todo_remove_tag(state: State<AppState>, todo_id: i64, tag_id: i64) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    tag_queries::remove_from_todo(&conn, todo_id, tag_id)?;
    todo_queries::get(&conn, todo_id)
}

#[tauri::command]
pub fn todo_list_by_tag(state: State<AppState>, tag_id: i64) -> AppResult<Vec<Todo>> {
    let conn = state.db.lock().unwrap();
    todo_queries::list_by_tag(&conn, tag_id)
}
