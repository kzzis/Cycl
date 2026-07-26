use dioxus::prelude::*;

use crate::hooks::use_tags::UseTags;
use crate::hooks::use_timings::UseTimings;

/// 設定画面。タグとタイミングの作成・削除をここに集約する。
#[component]
pub fn Settings() -> Element {
    let tags = use_context::<UseTags>();
    let timings = use_context::<UseTimings>();

    let mut tag_name = use_signal(String::new);
    let mut tag_color = use_signal(|| "#6366f1".to_string());
    let mut timing_name = use_signal(String::new);
    let mut timing_color = use_signal(|| "#6366f1".to_string());

    let submit_tag = move |event: FormEvent| {
        event.prevent_default();
        let trimmed = tag_name.read().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        tags.add(trimmed, tag_color.read().clone());
        tag_name.set(String::new());
    };

    let submit_timing = move |event: FormEvent| {
        event.prevent_default();
        let trimmed = timing_name.read().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        timings.add(trimmed, timing_color.read().clone());
        timing_name.set(String::new());
    };

    rsx! {
        div { class: "settings",
            section { class: "settings__section",
                h2 { class: "settings__title", "Tags" }
                form { class: "settings__form", onsubmit: submit_tag,
                    input {
                        r#type: "color",
                        class: "tag-bar__color",
                        value: "{tag_color}",
                        aria_label: "Tag color",
                        oninput: move |e| tag_color.set(e.value()),
                    }
                    input {
                        class: "tag-bar__name",
                        value: "{tag_name}",
                        placeholder: "New tag",
                        aria_label: "New tag name",
                        oninput: move |e| tag_name.set(e.value()),
                    }
                    button { class: "btn btn--primary", r#type: "submit", "Create" }
                }
                ul { class: "settings__list",
                    for tag in tags.items.read().iter().cloned() {
                        li { class: "settings__row",
                            span {
                                class: "tag-add__swatch",
                                style: "background-color: {tag.color}",
                            }
                            span { class: "settings__row-name", "{tag.name}" }
                            button {
                                class: "settings__delete",
                                aria_label: "Delete tag {tag.name}",
                                onclick: move |_| tags.remove(tag.id),
                                "✕"
                            }
                        }
                    }
                }
            }
            section { class: "settings__section",
                h2 { class: "settings__title", "Timings" }
                form { class: "settings__form", onsubmit: submit_timing,
                    input {
                        r#type: "color",
                        class: "tag-bar__color",
                        value: "{timing_color}",
                        aria_label: "Timing color",
                        oninput: move |e| timing_color.set(e.value()),
                    }
                    input {
                        class: "tag-bar__name",
                        value: "{timing_name}",
                        placeholder: "New timing",
                        aria_label: "New timing name",
                        oninput: move |e| timing_name.set(e.value()),
                    }
                    button { class: "btn btn--primary", r#type: "submit", "Create" }
                }
                ul { class: "settings__list",
                    for timing in timings.items.read().iter().cloned() {
                        li { class: "settings__row",
                            span {
                                class: "tag-add__swatch",
                                style: "background-color: {timing.color}",
                            }
                            span { class: "settings__row-name", "{timing.name}" }
                            if timing.is_builtin {
                                span { class: "settings__badge", "built-in" }
                            } else {
                                button {
                                    class: "settings__delete",
                                    aria_label: "Delete timing {timing.name}",
                                    onclick: move |_| timings.remove(timing.id),
                                    "✕"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
