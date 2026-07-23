use dioxus::prelude::*;
use shared::Todo;

#[component]
pub fn TodoItem(
    todo: Todo,
    on_toggle_complete: EventHandler<i64>,
    on_select_active: EventHandler<i64>,
    on_delete: EventHandler<i64>,
    on_drag_start: EventHandler<i64>,
    on_drop: EventHandler<i64>,
) -> Element {
    let target_label = todo
        .target_count
        .map(|target| format!(" / {target}"))
        .unwrap_or_default();
    let id = todo.id;

    rsx! {
        li {
            class: if todo.is_active { "todo-item todo-item--active" } else { "todo-item" },
            ondragover: move |e| e.prevent_default(),
            ondrop: move |e| {
                e.prevent_default();
                on_drop.call(id);
            },
            span {
                class: "todo-item__handle",
                draggable: "true",
                aria_label: "Drag to reorder {todo.title}",
                ondragstart: move |_| on_drag_start.call(id),
                "⠿"
            }
            input {
                r#type: "checkbox",
                checked: todo.is_completed,
                aria_label: "Mark {todo.title} as complete",
                onchange: move |_| on_toggle_complete.call(id),
            }
            button {
                class: if todo.is_completed { "todo-item__title todo-item__title--done" } else { "todo-item__title" },
                onclick: move |_| on_select_active.call(id),
                "{todo.title}"
            }
            span { class: "todo-item__count", "🍅×{todo.pomodoro_count}{target_label}" }
            button {
                class: "todo-item__delete",
                aria_label: "Delete {todo.title}",
                onclick: move |_| on_delete.call(id),
                "✕"
            }
        }
    }
}
