use crate::timer::engine::TimerEngine;
use shared::TimerState;
use tauri::State;

#[tauri::command]
pub fn timer_get_state(engine: State<TimerEngine>) -> TimerState {
    engine.snapshot()
}

#[tauri::command]
pub fn timer_start(engine: State<TimerEngine>) -> TimerState {
    engine.start()
}

#[tauri::command]
pub fn timer_pause(engine: State<TimerEngine>) -> TimerState {
    engine.pause()
}

#[tauri::command]
pub fn timer_reset(engine: State<TimerEngine>) -> TimerState {
    engine.reset()
}
