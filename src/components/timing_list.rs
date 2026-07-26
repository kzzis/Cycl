use dioxus::prelude::*;
use shared::{Timing, Todo};

#[component]
pub fn TimingList(
    timings: Vec<Timing>,
    todos: Vec<Todo>,
    on_select: EventHandler<Timing>,
    on_create: EventHandler<(String, String)>,
    on_delete: EventHandler<i64>,
) -> Element {
    let mut name = use_signal(String::new);
    let mut color = use_signal(|| "#6366f1".to_string());

    let submit = move |event: FormEvent| {
        event.prevent_default();
        let trimmed = name.read().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        on_create.call((trimmed, color.read().clone()));
        name.set(String::new());
    };

    rsx! {
        div { class: "timing-list",
            for timing in timings.iter().cloned() {
                {
                    let in_timing = todos.iter().filter(|t| t.category == timing.key);
                    let count = in_timing.clone().count();
                    let focus_mins: i64 = in_timing.map(|t| t.focus_secs).sum::<i64>() / 60;
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
                                span { class: "timing-row__focus", "{focus_mins}m" }
                                span { class: "timing-row__count", "{count}" }
                            }
                            if !timing.is_builtin {
                                button {
                                    class: "timing-row__delete",
                                    aria_label: "Delete timing {timing.name}",
                                    onclick: move |_| on_delete.call(timing.id),
                                    "✕"
                                }
                            }
                        }
                    }
                }
            }
            form { class: "timing-create", onsubmit: submit,
                input {
                    r#type: "color",
                    class: "tag-bar__color",
                    value: "{color}",
                    aria_label: "Timing color",
                    oninput: move |e| color.set(e.value()),
                }
                input {
                    class: "tag-bar__name",
                    value: "{name}",
                    placeholder: "New timing",
                    aria_label: "New timing name",
                    oninput: move |e| name.set(e.value()),
                }
                button { class: "btn btn--ghost", r#type: "submit", "Add" }
            }
        }
    }
}
