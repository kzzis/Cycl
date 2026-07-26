use dioxus::prelude::*;
use shared::CATEGORIES;

#[component]
pub fn CategoryTabs(active: Option<String>, on_select: EventHandler<Option<String>>) -> Element {
    rsx! {
        div { class: "category-tabs",
            button {
                class: if active.is_none() { "category-tab category-tab--active" } else { "category-tab" },
                onclick: move |_| on_select.call(None),
                "All"
            }
            for (value, label) in CATEGORIES.iter().copied() {
                button {
                    class: if active.as_deref() == Some(value) { "category-tab category-tab--active" } else { "category-tab" },
                    onclick: move |_| on_select.call(Some(value.to_string())),
                    "{label}"
                }
            }
        }
    }
}
