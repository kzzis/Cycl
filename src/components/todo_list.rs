use dioxus::prelude::*;

use super::{TodoForm, TodoItem};
use crate::hooks::use_todos::use_todos;

#[component]
pub fn TodoList() -> Element {
    let todos = use_todos();

    if *todos.is_loading.read() {
        return rsx! { p { class: "muted", "読み込み中..." } };
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
                    }
                }
            }
        }
    }
}
