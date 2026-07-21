#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::components::{PomodoroTimer, TodoList};

static CSS: Asset = asset!("/assets/styles.css");

pub fn App() -> Element {
    rsx! {
        link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=Space+Grotesk:wght@500;600;700&display=swap",
        }
        link { rel: "stylesheet", href: CSS }
        main { class: "app",
            header { class: "app__header",
                span { class: "app__logo", "🍅" }
                h1 { class: "app__title", "Cycl" }
            }
            div { class: "app__panels",
                PomodoroTimer {}
                TodoList {}
            }
        }
    }
}
