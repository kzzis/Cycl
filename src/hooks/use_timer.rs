use dioxus::prelude::*;
use shared::TimerState;

use crate::tauri_api::{self, timer as api};

#[derive(Clone, Copy)]
pub struct UseTimer {
    pub state: Signal<Option<TimerState>>,
}

impl UseTimer {
    pub fn start(&self) {
        spawn(async {
            let _ = api::start_timer().await;
        });
    }

    pub fn pause(&self) {
        spawn(async {
            let _ = api::pause_timer().await;
        });
    }

    pub fn reset(&self) {
        spawn(async {
            let _ = api::reset_timer().await;
        });
    }
}

pub fn use_timer() -> UseTimer {
    let hook = UseTimer {
        state: use_signal(|| None),
    };

    use_effect(move || {
        let mut initial_state = hook.state;
        spawn(async move {
            if let Ok(initial) = api::get_timer_state().await {
                initial_state.set(Some(initial));
            }
        });

        let mut ticked_state = hook.state;
        tauri_api::listen::<TimerState>("timer:tick", move |snapshot| {
            ticked_state.set(Some(snapshot));
        });
    });

    hook
}
