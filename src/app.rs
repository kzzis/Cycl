#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::components::{PomodoroTimer, TodoList};

static CSS: Asset = asset!("/assets/styles.css");

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Timer,
    Tasks,
}

pub fn App() -> Element {
    let mut tab = use_signal(|| Tab::Timer);
    let active = *tab.read();

    rsx! {
        link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Space+Grotesk:wght@500;600;700&display=swap",
        }
        link { rel: "stylesheet", href: CSS }
        main { class: "app",
            // タブバーはウィンドウのドラッグ領域も兼ねる(ボタンは自動で除外される)。
            div { class: "app__tabs", "data-tauri-drag-region": "deep",
                button {
                    class: if active == Tab::Timer { "app__tab app__tab--active" } else { "app__tab" },
                    onclick: move |_| tab.set(Tab::Timer),
                    "Timer"
                }
                button {
                    class: if active == Tab::Tasks { "app__tab app__tab--active" } else { "app__tab" },
                    onclick: move |_| tab.set(Tab::Tasks),
                    "Tasks"
                }
            }
            div { class: "app__content",
                match active {
                    Tab::Timer => rsx! { PomodoroTimer {} },
                    Tab::Tasks => rsx! { TodoList {} },
                }
            }
        }
    }
}
