use dioxus::prelude::*;
use shared::{format_mm_ss, phase_label};

use crate::hooks::use_timer::UseTimer;

/// Tasks画面の上部に出す、動作中タイマーのライブ表示。停止中は何も出さない。
#[component]
pub fn TimerStatus() -> Element {
    let timer = use_context::<UseTimer>();
    let Some(state) = timer.state.read().clone() else {
        return rsx! {};
    };
    if !state.is_running {
        return rsx! {};
    }

    rsx! {
        div { class: "timer-status",
            span { class: "timer-status__phase", "{phase_label(state.phase)}" }
            span { class: "timer-status__time", "{format_mm_ss(state.remaining_secs)}" }
        }
    }
}
