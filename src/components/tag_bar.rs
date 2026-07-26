use dioxus::prelude::*;
use shared::Tag;

#[component]
pub fn TagBar(
    tags: Vec<Tag>,
    active_filter: Option<i64>,
    on_filter: EventHandler<Option<i64>>,
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
        div { class: "tag-bar",
            div { class: "tag-bar__filters",
                button {
                    class: if active_filter.is_none() { "tag-filter tag-filter--active" } else { "tag-filter" },
                    onclick: move |_| on_filter.call(None),
                    "All"
                }
                for tag in tags.iter().cloned() {
                    span {
                        class: if active_filter == Some(tag.id) { "tag-filter tag-filter--active" } else { "tag-filter" },
                        style: "border-color: {tag.color}",
                        button {
                            class: "tag-filter__label",
                            onclick: move |_| on_filter.call(Some(tag.id)),
                            span {
                                class: "tag-add__swatch",
                                style: "background-color: {tag.color}",
                            }
                            "{tag.name}"
                        }
                        button {
                            class: "tag-filter__delete",
                            aria_label: "Delete tag {tag.name}",
                            onclick: move |_| on_delete.call(tag.id),
                            "✕"
                        }
                    }
                }
            }
            form { class: "tag-bar__create", onsubmit: submit,
                input {
                    r#type: "color",
                    class: "tag-bar__color",
                    value: "{color}",
                    aria_label: "Tag color",
                    oninput: move |e| color.set(e.value()),
                }
                input {
                    class: "tag-bar__name",
                    value: "{name}",
                    placeholder: "New tag",
                    aria_label: "New tag name",
                    oninput: move |e| name.set(e.value()),
                }
                button { class: "btn btn--ghost", r#type: "submit", "Create" }
            }
        }
    }
}
