use shared::{TimerSettings, TimerState};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::time::{interval, Duration};

pub struct TimerEngine {
    state: Arc<Mutex<TimerState>>,
}

impl TimerEngine {
    pub fn new(app_handle: AppHandle) -> Self {
        let state = Arc::new(Mutex::new(TimerState::new(TimerSettings::default())));
        spawn_tick_loop(app_handle, state.clone());
        TimerEngine { state }
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
        state.is_running = false;
        state.clone()
    }

    pub fn reset(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        state.reset_current_phase();
        state.clone()
    }
}

fn spawn_tick_loop(app_handle: AppHandle, state: Arc<Mutex<TimerState>>) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;

            let snapshot = {
                let mut state = state.lock().unwrap();
                if state.is_running {
                    if state.remaining_secs > 0 {
                        state.remaining_secs -= 1;
                    }
                    if state.remaining_secs == 0 {
                        state.advance_phase();
                    }
                }
                state.clone()
            };

            let _ = app_handle.emit("timer:tick", &snapshot);
        }
    });
}
