#![warn(clippy::all)]

mod commands;
mod db;
mod error;
mod models;
mod timer;
mod tray;

use db::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use timer::engine::TimerEngine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("cycl.sqlite3");
            let conn = db::open(&db_path)?;
            let db = Arc::new(Mutex::new(conn));

            app.manage(AppState { db: db.clone() });
            app.manage(TimerEngine::new(app.handle().clone(), db.clone()));
            tray::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::todo::todo_list,
            commands::todo::todo_create,
            commands::todo::todo_update,
            commands::todo::todo_delete,
            commands::todo::todo_toggle_complete,
            commands::todo::todo_set_active,
            commands::todo::todo_reorder,
            commands::todo::todo_list_by_category,
            commands::todo::todo_update_category,
            commands::timer::timer_get_state,
            commands::timer::timer_start,
            commands::timer::timer_pause,
            commands::timer::timer_reset,
            commands::tag::tag_list,
            commands::tag::tag_create,
            commands::tag::tag_delete,
            commands::tag::todo_add_tag,
            commands::tag::todo_remove_tag,
            commands::tag::todo_list_by_tag,
            commands::timing::timing_list,
            commands::timing::timing_create,
            commands::timing::timing_delete,
            commands::stats::stats_focus_by_tag,
            commands::stats::stats_monthly,
            commands::stats::stats_accuracy,
            commands::stats::stats_focus_hours,
            commands::stats::stats_tag_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
