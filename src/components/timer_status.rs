use dioxus::prelude::*;
use shared::{format_mm_ss, phase_label, TimerPhase};

use crate::hooks::use_timer::UseTimer;
use crate::hooks::use_todos::UseTodos;

/// Tasks画面の上部に出す、動作中タイマーのライブ表示。停止中は何も出さない。
#[component]
pub fn TimerStatus() -> Element {
    let timer = use_context::<UseTimer>();
    let todos = use_context::<UseTodos>();
    let Some(state) = timer.state.read().clone() else {
        return rsx! {};
    };
    if !state.is_running {
        return rsx! {};
    }

    // 作業フェーズ中は取り組み中タスク名を、それ以外はフェーズ名を表示する。
    let active_title = todos
        .items
        .read()
        .iter()
        .find(|t| t.is_active)
        .map(|t| t.title.clone());
    let label = match (state.phase, active_title) {
        (TimerPhase::Work, Some(title)) => title,
        (phase, _) => phase_label(phase).to_string(),
    };

    rsx! {
        div { class: "timer-status",
            span { class: "timer-status__phase", "{label}" }
            span { class: "timer-status__time", "{format_mm_ss(state.remaining_secs)}" }
        }
    }
}
