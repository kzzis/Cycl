use dioxus::prelude::*;
use shared::{Tag, Timing, Todo};

#[component]
pub fn TodoItem(
    todo: Todo,
    all_tags: Vec<Tag>,
    all_timings: Vec<Timing>,
    is_dragging: bool,
    on_toggle_complete: EventHandler<i64>,
    on_select_active: EventHandler<i64>,
    on_delete: EventHandler<i64>,
    on_drag_start: EventHandler<i64>,
    on_hover: EventHandler<i64>,
    on_add_tag: EventHandler<(i64, i64)>,
    on_remove_tag: EventHandler<(i64, i64)>,
    on_change_category: EventHandler<(i64, String)>,
) -> Element {
    let target_label = todo
        .target_count
        .map(|target| format!(" / {target}"))
        .unwrap_or_default();
    let id = todo.id;
    let mut class = if todo.is_active {
        "todo-item todo-item--active".to_string()
    } else {
        "todo-item".to_string()
    };
    if is_dragging {
        class.push_str(" todo-item--dragging");
    }

    let mut show_menu = use_signal(|| false);
    // このTodoにまだ付いていないタグだけを追加候補に出す。
    let available: Vec<Tag> = all_tags
        .iter()
        .filter(|t| !todo.tags.iter().any(|assigned| assigned.id == t.id))
        .cloned()
        .collect();

    rsx! {
        li {
            class,
            onmouseenter: move |_| on_hover.call(id),
            div { class: "todo-item__main",
                span {
                    class: "todo-item__handle",
                    aria_label: "Drag to reorder {todo.title}",
                    onmousedown: move |e| {
                        e.prevent_default();
                        on_drag_start.call(id);
                    },
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
                select {
                    class: "todo-item__category",
                    aria_label: "Timing for {todo.title}",
                    value: "{todo.category}",
                    onchange: move |e| on_change_category.call((id, e.value())),
                    for timing in all_timings.iter().cloned() {
                        option {
                            value: "{timing.key}",
                            selected: todo.category == timing.key,
                            "{timing.name}"
                        }
                    }
                }
                button {
                    class: "todo-item__delete",
                    aria_label: "Delete {todo.title}",
                    onclick: move |_| on_delete.call(id),
                    "✕"
                }
            }
            div { class: "todo-item__tags",
                for tag in todo.tags.iter().cloned() {
                    span {
                        class: "tag-chip",
                        style: "background-color: {tag.color}",
                        "{tag.name}"
                        button {
                            class: "tag-chip__remove",
                            aria_label: "Remove tag {tag.name}",
                            onclick: move |_| on_remove_tag.call((id, tag.id)),
                            "✕"
                        }
                    }
                }
                if !available.is_empty() {
                    div { class: "tag-add",
                        button {
                            class: "tag-add__button",
                            aria_label: "Add tag",
                            onclick: move |_| {
                                let current = *show_menu.read();
                                show_menu.set(!current);
                            },
                            "+ tag"
                        }
                        if *show_menu.read() {
                            div { class: "tag-add__menu",
                                for tag in available.iter().cloned() {
                                    button {
                                        class: "tag-add__option",
                                        onclick: move |_| {
                                            on_add_tag.call((id, tag.id));
                                            show_menu.set(false);
                                        },
                                        span {
                                            class: "tag-add__swatch",
                                            style: "background-color: {tag.color}",
                                        }
                                        "{tag.name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
