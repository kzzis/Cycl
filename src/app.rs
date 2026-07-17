#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::components::TodoList;

static CSS: Asset = asset!("/assets/styles.css");

pub fn App() -> Element {
    rsx! {
        link { rel: "stylesheet", href: CSS }
        main { class: "app",
            TodoList {}
        }
    }
}
