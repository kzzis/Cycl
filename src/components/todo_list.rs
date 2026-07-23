use dioxus::prelude::*;

use super::{TodoForm, TodoItem};
use crate::hooks::use_todos::use_todos;

#[component]
pub fn TodoList() -> Element {
    let todos = use_todos();
    let mut dragging_id = use_signal(|| None::<i64>);

    if *todos.is_loading.read() {
        return rsx! { p { class: "muted", "Loading..." } };
    }

    rsx! {
        div { class: "todo-list",
            TodoForm {
                on_submit: move |(title, target_count): (String, Option<i64>)| {
                    todos.add(title, target_count);
                }
            }
            ul {
                for todo in todos.items.read().iter().cloned() {
                    TodoItem {
                        key: "{todo.id}",
                        todo: todo.clone(),
                        on_toggle_complete: move |id| todos.toggle_complete(id),
                        on_select_active: move |id| todos.select_active(id),
                        on_delete: move |id| todos.remove(id),
                        on_drag_start: move |id| dragging_id.set(Some(id)),
                        on_drop: move |drop_on_id: i64| {
                            let Some(dragged_id) = dragging_id.take() else {
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
                            let to = ids.iter().position(|&id| id == drop_on_id).unwrap_or(ids.len());
                            ids.insert(to, dragged_id);
                            todos.reorder(ids);
                        },
                    }
                }
            }
        }
    }
}
