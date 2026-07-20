use crate::db::{session_queries, todo_queries};
use chrono::Utc;
use rusqlite::Connection;
use shared::{format_mm_ss, TimerPhase, TimerSettings, TimerState};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::time::{interval, Duration};

pub struct TimerEngine {
    state: Arc<Mutex<TimerState>>,
    session_started_at: Arc<Mutex<Option<String>>>,
}

impl TimerEngine {
    pub fn new(app_handle: AppHandle, db: Arc<Mutex<Connection>>) -> Self {
        let state = Arc::new(Mutex::new(TimerState::new(TimerSettings::default())));
        let session_started_at = Arc::new(Mutex::new(None));
        spawn_tick_loop(app_handle, state.clone(), db, session_started_at.clone());
        TimerEngine {
            state,
            session_started_at,
        }
    }

    pub fn snapshot(&self) -> TimerState {
        self.state.lock().unwrap().clone()
    }

    pub fn start(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        if state.phase == TimerPhase::Work && !state.is_running {
            let mut started_at = self.session_started_at.lock().unwrap();
            if started_at.is_none() {
                *started_at = Some(Utc::now().to_rfc3339());
            }
        }
        state.is_running = true;
        state.clone()
    }

    pub fn pause(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        state.is_running = false;
        state.clone()
    }

    pub fn reset(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        state.reset_current_phase();
        *self.session_started_at.lock().unwrap() = None;
        state.clone()
    }
}

fn spawn_tick_loop(
    app_handle: AppHandle,
    state: Arc<Mutex<TimerState>>,
    db: Arc<Mutex<Connection>>,
    session_started_at: Arc<Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;

            let mut just_completed_work = false;
            let snapshot = {
                let mut state = state.lock().unwrap();
                if state.is_running {
                    if state.remaining_secs > 0 {
                        state.remaining_secs -= 1;
                    }
                    if state.remaining_secs == 0 {
                        just_completed_work = state.phase == TimerPhase::Work;
                        state.advance_phase();
                    }
                }
                state.clone()
            };

            if just_completed_work {
                if let Some(started_at) = session_started_at.lock().unwrap().take() {
                    record_work_session(&app_handle, &db, &started_at);
                }
            }

            let title = snapshot
                .is_running
                .then(|| format_mm_ss(snapshot.remaining_secs));
            crate::tray::update_title(&app_handle, title);

            let _ = app_handle.emit("timer:tick", &snapshot);
        }
    });
}

/// 作業セッションが1本終わった瞬間の後処理。
/// アクティブなTodoが選ばれていなければ何もしない(記録も通知もしない)。
fn record_work_session(app_handle: &AppHandle, db: &Arc<Mutex<Connection>>, started_at: &str) {
    let conn = db.lock().unwrap();

    let Ok(Some(todo)) = todo_queries::get_active(&conn) else {
        return;
    };
    if session_queries::record_completed(&conn, todo.id, started_at).is_err() {
        return;
    }
    let Ok(updated) = todo_queries::increment_pomodoro_count(&conn, todo.id) else {
        return;
    };

    drop(conn); // 通知呼び出しの前にロックを解放する

    let _ = app_handle.emit("todos:changed", ());

    let _ = app_handle
        .notification()
        .builder()
        .title("ポモドーロ完了")
        .body(format!(
            "「{}」を1セット完了しました 🍅×{}",
            updated.title, updated.pomodoro_count
        ))
        .show();
}
