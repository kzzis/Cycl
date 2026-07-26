use dioxus::prelude::*;
use wasm_bindgen::{closure::Closure, JsCast};

const FORM_ID: &str = "todo-add-form";

/// フォーカスがフォームの外へ出たら閉じる。入力欄間の移動では閉じないよう、
/// 次のイベントループで`activeElement`がフォーム内かを確認してから判定する。
fn close_when_focus_left(mut is_open: Signal<bool>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb = Closure::once_into_js(move || {
        let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let still_inside = doc
            .get_element_by_id(FORM_ID)
            .map(|form| form.contains(doc.active_element().as_deref()))
            .unwrap_or(false);
        if !still_inside {
            is_open.set(false);
        }
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 0);
}

#[component]
pub fn TodoForm(on_submit: EventHandler<(String, Option<i64>)>) -> Element {
    let mut title = use_signal(String::new);
    let mut target_count = use_signal(|| "1".to_string());
    let mut is_open = use_signal(|| false);

    let submit = move |event: FormEvent| {
        event.prevent_default();
        let trimmed = title.read().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let parsed_target = if target_count.read().trim().is_empty() {
            None
        } else {
            target_count.read().parse::<i64>().ok()
        };
        on_submit.call((trimmed, parsed_target));
        title.set(String::new());
        target_count.set("1".to_string());
    };

    // 普段は「+」だけ表示。押すと入力欄を開く。
    if !*is_open.read() {
        return rsx! {
            button {
                class: "todo-form__toggle",
                aria_label: "Add todo",
                onclick: move |_| is_open.set(true),
                span { class: "todo-form__toggle-icon", "+" }
                "Add todo"
            }
        };
    }

    rsx! {
        form {
            id: FORM_ID,
            class: "todo-form",
            onsubmit: submit,
            // フォーカスがフォーム外へ出たら閉じる(状態は保持)。
            onfocusout: move |_| close_when_focus_left(is_open),
            input {
                value: "{title}",
                placeholder: "New todo",
                aria_label: "Todo title",
                autofocus: true,
                oninput: move |e| title.set(e.value()),
            }
            input {
                value: "{target_count}",
                r#type: "number",
                min: "0",
                placeholder: "Target 🍅",
                aria_label: "Target pomodoro count",
                oninput: move |e| target_count.set(e.value()),
            }
            button {
                class: "todo-form__add",
                r#type: "submit",
                aria_label: "Add todo",
                disabled: title.read().trim().is_empty(),
                "+"
            }
        }
    }
}
