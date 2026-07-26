use dioxus::prelude::*;
use shared::Tag;

/// タスク詳細のタグ絞り込みバー。作成・削除は設定画面に集約したので、ここは選択専用。
#[component]
pub fn TagBar(
    tags: Vec<Tag>,
    active_filter: Option<i64>,
    on_filter: EventHandler<Option<i64>>,
) -> Element {
    rsx! {
        div { class: "tag-bar",
            div { class: "tag-bar__filters",
                button {
                    class: if active_filter.is_none() { "tag-filter tag-filter--active" } else { "tag-filter" },
                    onclick: move |_| on_filter.call(None),
                    "All"
                }
                for tag in tags.iter().cloned() {
                    button {
                        class: if active_filter == Some(tag.id) { "tag-filter tag-filter--active" } else { "tag-filter" },
                        onclick: move |_| on_filter.call(Some(tag.id)),
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
