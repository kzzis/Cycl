use dioxus::prelude::*;
use shared::Timing;

use super::{TimerStatus, TimingList, TodoList};
use crate::hooks::use_timings::UseTimings;
use crate::hooks::use_todos::UseTodos;

/// Tasksタブ本体。タイミング一覧(マスター)と、選択したタイミングのTodo詳細を切り替える。
#[component]
pub fn TasksView() -> Element {
    let timings = use_context::<UseTimings>();
    let todos = use_context::<UseTodos>();
    let mut selected = use_signal(|| None::<Timing>);

    let current = selected.read().clone();

    rsx! {
        div { class: "tasks-view",
            TimerStatus {}
            match current {
                Some(timing) => rsx! {
                    TodoList {
                        timing,
                        on_back: move |_| {
                            // 詳細で追加/変更された内容をマスターの件数に反映するため取り直す。
                            todos.refresh();
                            selected.set(None);
                        },
                    }
                },
                None => rsx! {
                    TimingList {
                        timings: timings.items.read().clone(),
                        todos: todos.items.read().clone(),
                        on_select: move |t: Timing| selected.set(Some(t)),
                    }
                },
            }
        }
    }
}
