use dioxus::prelude::*;
use shared::{format_mm_ss, phase_label, TimerPhase};

use crate::hooks::use_timer::use_timer;
use crate::hooks::use_todos::use_todos;

const RADIUS: f64 = 90.0;

#[component]
pub fn PomodoroTimer() -> Element {
    let timer = use_timer();
    let todos = use_todos();
    let Some(state) = timer.state.read().clone() else {
        return rsx! { p { class: "muted", "Loading..." } };
    };

    let circumference = 2.0 * std::f64::consts::PI * RADIUS;
    let total = state.settings.duration_secs(state.phase) as f64;
    let progress = if total == 0.0 {
        0.0
    } else {
        (total - state.remaining_secs as f64) / total
    };
    let offset = circumference * (1.0 - progress);

    // 作業フェーズ中は取り組み中タスク名を、それ以外はフェーズ名を表示する。
    let active_title = todos
        .items
        .read()
        .iter()
        .find(|t| t.is_active)
        .map(|t| t.title.clone());
    let phase_text = match (state.phase, active_title) {
        (TimerPhase::Work, Some(title)) => title,
        (phase, _) => phase_label(phase).to_string(),
    };

    rsx! {
        div { class: "pomodoro",
            p { class: "pomodoro__phase", "{phase_text}" }
            div { class: "pomodoro__ring",
                svg {
                    width: "220", height: "220", view_box: "0 0 220 220",
                    class: "pomodoro__svg",
                    defs {
                        linearGradient {
                            id: "pomodoro-gradient",
                            x1: "0%", y1: "0%", x2: "100%", y2: "100%",
                            stop { offset: "0%", stop_color: "#818cf8" }
                            stop { offset: "100%", stop_color: "#c084fc" }
                        }
                    }
                    circle {
                        cx: "110", cy: "110", r: "{RADIUS}",
                        fill: "none", stroke: "currentColor", stroke_width: "12",
                        class: "pomodoro__track",
                    }
                    circle {
                        cx: "110", cy: "110", r: "{RADIUS}",
                        fill: "none", stroke: "url(#pomodoro-gradient)", stroke_width: "12",
                        stroke_linecap: "round",
                        stroke_dasharray: "{circumference}",
                        stroke_dashoffset: "{offset}",
                        class: "pomodoro__progress",
                    }
                }
                span { class: "pomodoro__remaining", "{format_mm_ss(state.remaining_secs)}" }
            }
            div { class: "pomodoro__controls",
                if state.is_running {
                    button { class: "btn btn--primary", onclick: move |_| timer.pause(), "Pause" }
                } else {
                    button { class: "btn btn--primary", onclick: move |_| timer.start(), "Start" }
                }
                button { class: "btn btn--ghost", onclick: move |_| timer.reset(), "Reset" }
            }
        }
    }
}
