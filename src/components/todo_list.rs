use dioxus::prelude::*;

use super::{TodoForm, TodoItem};
use crate::hooks::use_todos::use_todos;

#[component]
pub fn TodoList() -> Element {
    let todos = use_todos();
    let mut dragging_id = use_signal(|| None::<i64>);
    let mut hover_id = use_signal(|| None::<i64>);

    if *todos.is_loading.read() {
        return rsx! { p { class: "muted", "Loading..." } };
    }

    // ドラッグ確定: ドラッグ中のidを、最後にホバーした行の位置へ移動する。
    let commit_drop = move |_| {
        let Some(dragged_id) = dragging_id.take() else {
            return;
        };
        let Some(drop_on_id) = hover_id.take() else {
            return;
        };
        if dragged_id == drop_on_id {
            return;
        }
        let mut ids: Vec<i64> = todos.items.read().iter().map(|t| t.id).collect();
        let Some(from) = ids.iter().position(|&id| id == dragged_id) else {
            return;
        };
        ids.remove(from);
        let to = ids
            .iter()
            .position(|&id| id == drop_on_id)
            .unwrap_or(ids.len());
        ids.insert(to, dragged_id);
        todos.reorder(ids);
    };

    rsx! {
        div { class: "todo-list",
            TodoForm {
                on_submit: move |(title, target_count): (String, Option<i64>)| {
                    todos.add(title, target_count);
                }
            }
            ul {
                onmouseup: commit_drop,
                onmouseleave: move |_| {
                    dragging_id.set(None);
                    hover_id.set(None);
                },
                for todo in todos.items.read().iter().cloned() {
                    TodoItem {
                        key: "{todo.id}",
                        todo: todo.clone(),
                        is_dragging: *dragging_id.read() == Some(todo.id),
                        on_toggle_complete: move |id| todos.toggle_complete(id),
                        on_select_active: move |id| todos.select_active(id),
                        on_delete: move |id| todos.remove(id),
                        on_drag_start: move |id| dragging_id.set(Some(id)),
                        on_hover: move |id| {
                            if dragging_id.read().is_some() {
                                hover_id.set(Some(id));
                            }
                        },
                    }
                }
            }
        }
    }
}
