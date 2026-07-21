use dioxus::prelude::*;

#[component]
pub fn TodoForm(on_submit: EventHandler<(String, Option<i64>)>) -> Element {
    let mut title = use_signal(String::new);
    let mut target_count = use_signal(String::new);

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
        target_count.set(String::new());
    };

    rsx! {
        form { class: "todo-form", onsubmit: submit,
            input {
                value: "{title}",
                placeholder: "New todo",
                aria_label: "Todo title",
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
            button { class: "btn btn--primary", r#type: "submit", "Add" }
        }
    }
}
