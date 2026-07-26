use dioxus::prelude::*;
use shared::{format_focus, Timing, Todo};

/// Tasksタブのマスター。タイミングの選択に専念する(作成・削除は設定画面へ)。
#[component]
pub fn TimingList(
    timings: Vec<Timing>,
    todos: Vec<Todo>,
    on_select: EventHandler<Timing>,
) -> Element {
    rsx! {
        div { class: "timing-list",
            for timing in timings.iter().cloned() {
                {
                    let in_timing = todos.iter().filter(|t| t.category == timing.key);
                    let count = in_timing.clone().count();
                    let focus_label = format_focus(in_timing.map(|t| t.focus_secs).sum());
                    let selected = timing.clone();
                    rsx! {
                        div { class: "timing-row",
                            button {
                                class: "timing-row__open",
                                onclick: move |_| on_select.call(selected.clone()),
                                span {
                                    class: "timing-row__dot",
                                    style: "background-color: {timing.color}",
                                }
                                span { class: "timing-row__name", "{timing.name}" }
                                span { class: "timing-row__focus", "{focus_label}" }
                                span { class: "timing-row__count", "{count}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
