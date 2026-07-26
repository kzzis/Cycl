#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::components::{PomodoroTimer, TasksView};

static CSS: Asset = asset!("/assets/styles.css");

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Timer,
    Tasks,
}

pub fn App() -> Element {
    let mut tab = use_signal(|| Tab::Timer);
    // タブ状態をコンテキストで共有し、深い階層(タスク行の再生ボタン等)からも切り替えられるようにする。
    use_context_provider(|| tab);
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
            // 両ビューを常にマウントしたままにし、非アクティブ側をCSSで隠す。
            // こうするとタブを切り替えても各ビューのローカル状態が保持される。
            div { class: "app__content",
                div {
                    class: if active == Tab::Timer { "tab-panel" } else { "tab-panel tab-panel--hidden" },
                    PomodoroTimer {}
                }
                div {
                    class: if active == Tab::Tasks { "tab-panel" } else { "tab-panel tab-panel--hidden" },
                    TasksView {}
                }
            }
        }
    }
}
