#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::components::{PomodoroTimer, Settings, Stats, TasksView};
use crate::hooks::use_tags::use_tags;
use crate::hooks::use_timer::use_timer;
use crate::hooks::use_timings::use_timings;
use crate::hooks::use_todos::use_todos;

static CSS: Asset = asset!("/assets/styles.css");

#[derive(Clone, Copy, PartialEq)]
pub enum Tab {
    Timer,
    Tasks,
    Stats,
    Settings,
}

pub fn App() -> Element {
    let mut tab = use_signal(|| Tab::Timer);
    let todos = use_todos();
    let timer = use_timer();
    let tags = use_tags();
    let timings = use_timings();
    // タブ状態や各種状態をコンテキストで共有し、全ビューが同じ状態を見るようにする。
    // (取り組み中タスクやタグ/タイミングの変更がビュー間で即座に反映される)
    use_context_provider(|| tab);
    use_context_provider(|| todos);
    use_context_provider(|| timer);
    use_context_provider(|| tags);
    use_context_provider(|| timings);
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
                button {
                    class: if active == Tab::Stats { "app__tab app__tab--active" } else { "app__tab" },
                    onclick: move |_| tab.set(Tab::Stats),
                    "Stats"
                }
                button {
                    class: if active == Tab::Settings { "app__gear app__gear--active" } else { "app__gear" },
                    aria_label: "Settings",
                    onclick: move |_| tab.set(Tab::Settings),
                    "⚙"
                }
            }
            // 各ビューを常にマウントしたままにし、非アクティブ側をCSSで隠す。
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
                div {
                    class: if active == Tab::Stats { "tab-panel" } else { "tab-panel tab-panel--hidden" },
                    Stats {}
                }
                div {
                    class: if active == Tab::Settings { "tab-panel" } else { "tab-panel tab-panel--hidden" },
                    Settings {}
                }
            }
        }
    }
}
