use crate::db::{session_queries, todo_queries};
use chrono::Utc;
use rusqlite::Connection;
use shared::{format_mm_ss, TimerPhase, TimerSettings, TimerState};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::time::{interval, Duration};

/// まだDBへ書き出していない、取り組み中タスクの作業経過(秒)。
#[derive(Default)]
struct PendingFocus {
    todo_id: Option<i64>,
    secs: i64,
}

pub struct TimerEngine {
    state: Arc<Mutex<TimerState>>,
    pending: Arc<Mutex<PendingFocus>>,
    /// 現在の作業フェーズ中に発生した中断(一時停止・タスク切替)の回数。
    /// フェーズ完走時にセッションへ書き出してリセットする。
    phase_interruptions: Arc<Mutex<i64>>,
    db: Arc<Mutex<Connection>>,
    app_handle: AppHandle,
}

impl TimerEngine {
    pub fn new(app_handle: AppHandle, db: Arc<Mutex<Connection>>) -> Self {
        let state = Arc::new(Mutex::new(TimerState::new(TimerSettings::default())));
        let pending = Arc::new(Mutex::new(PendingFocus::default()));
        let phase_interruptions = Arc::new(Mutex::new(0));
        spawn_tick_loop(
            app_handle.clone(),
            state.clone(),
            db.clone(),
            pending.clone(),
            phase_interruptions.clone(),
        );
        TimerEngine {
            state,
            pending,
            phase_interruptions,
            db,
            app_handle,
        }
    }

    pub fn snapshot(&self) -> TimerState {
        self.state.lock().unwrap().clone()
    }

    pub fn start(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        state.is_running = true;
        state.clone()
    }

    pub fn pause(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        // 作業中の一時停止は中断1回として数える。
        if state.is_running && state.phase == TimerPhase::Work {
            *self.phase_interruptions.lock().unwrap() += 1;
        }
        state.is_running = false;
        drop(state);
        // 途中経過を記録してから停止状態を返す。
        flush_focus(&self.pending, &self.db, &self.app_handle, false, 0);
        self.state.lock().unwrap().clone()
    }

    pub fn reset(&self) -> TimerState {
        flush_focus(&self.pending, &self.db, &self.app_handle, false, 0);
        let mut state = self.state.lock().unwrap();
        state.reset_current_phase();
        // フェーズをやり直すので中断カウントも破棄する。
        *self.phase_interruptions.lock().unwrap() = 0;
        state.clone()
    }

    /// 取り組み中タスクの切り替え前などに、溜まっている作業経過をDBへ書き出す。
    pub fn flush_focus(&self) {
        // 作業中のタスク切り替えも中断1回として数える。
        {
            let state = self.state.lock().unwrap();
            if state.is_running && state.phase == TimerPhase::Work {
                *self.phase_interruptions.lock().unwrap() += 1;
            }
        }
        flush_focus(&self.pending, &self.db, &self.app_handle, false, 0);
    }
}

/// 溜まっている作業経過を、作業チャンク(セッション)としてDBに記録し、
/// タスクの累積作業時間へ加算する。`completed`は1ポモドーロ完走かどうかで、
/// `interruptions`は完走した行にだけ記録する中断回数。
fn flush_focus(
    pending: &Arc<Mutex<PendingFocus>>,
    db: &Arc<Mutex<Connection>>,
    app: &AppHandle,
    completed: bool,
    interruptions: i64,
) {
    let mut p = pending.lock().unwrap();
    if p.secs > 0 {
        if let Some(id) = p.todo_id {
            {
                let conn = db.lock().unwrap();
                let now = Utc::now().to_rfc3339();
                let _ = session_queries::record_focus(
                    &conn,
                    id,
                    &now,
                    p.secs,
                    completed,
                    interruptions,
                );
                let _ = todo_queries::add_focus_secs(&conn, id, p.secs);
            }
            let _ = app.emit("todos:changed", ());
        }
    }
    p.todo_id = None;
    p.secs = 0;
}

fn spawn_tick_loop(
    app_handle: AppHandle,
    state: Arc<Mutex<TimerState>>,
    db: Arc<Mutex<Connection>>,
    pending: Arc<Mutex<PendingFocus>>,
    phase_interruptions: Arc<Mutex<i64>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;

            let mut just_completed_work = false;
            let mut accrue_work = false;
            let snapshot = {
                let mut state = state.lock().unwrap();
                if state.is_running {
                    // このtickで作業フェーズの1秒を消費したか。
                    accrue_work = state.phase == TimerPhase::Work;
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

            // 作業した1秒を、取り組み中タスク宛ての未書き出し分として積む。
            if accrue_work {
                let mut p = pending.lock().unwrap();
                if p.todo_id.is_none() {
                    let conn = db.lock().unwrap();
                    p.todo_id = todo_queries::get_active(&conn).ok().flatten().map(|t| t.id);
                }
                if p.todo_id.is_some() {
                    p.secs += 1;
                }
            }

            if just_completed_work {
                // 完走分は、このフェーズ中の中断回数と一緒にチャンク記録する。
                let interruptions = std::mem::take(&mut *phase_interruptions.lock().unwrap());
                flush_focus(&pending, &db, &app_handle, true, interruptions);
                notify_completion(&app_handle, &db);
            }

            let title = snapshot
                .is_running
                .then(|| format_mm_ss(snapshot.remaining_secs));
            crate::tray::update_title(&app_handle, title);

            let _ = app_handle.emit("timer:tick", &snapshot);
        }
    });
}

/// 作業セッション完走時に、取り組み中タスクのポモドーロ数を増やして通知する。
fn notify_completion(app_handle: &AppHandle, db: &Arc<Mutex<Connection>>) {
    let conn = db.lock().unwrap();
    let Ok(Some(todo)) = todo_queries::get_active(&conn) else {
        return;
    };
    let Ok(updated) = todo_queries::increment_pomodoro_count(&conn, todo.id) else {
        return;
    };
    drop(conn); // 通知呼び出しの前にロックを解放する

    let _ = app_handle.emit("todos:changed", ());

    let _ = app_handle
        .notification()
        .builder()
        .title("Pomodoro Complete")
        .body(format!(
            "\"{}\" finished a session 🍅×{}",
            updated.title, updated.pomodoro_count
        ))
        .show();
}
