use dioxus::prelude::*;
use shared::Timing;

use crate::tauri_api::timing as api;

#[derive(Clone, Copy)]
pub struct UseTimings {
    pub items: Signal<Vec<Timing>>,
}

impl UseTimings {
    pub fn refresh(&self) {
        let mut items = self.items;
        spawn(async move {
            if let Ok(list) = api::list_timings().await {
                items.set(list);
            }
        });
    }

    pub fn add(&self, name: String, color: String) {
        let this = *self;
        spawn(async move {
            if api::create_timing(&name, &color).await.is_ok() {
                this.refresh();
            }
        });
    }

    pub fn remove(&self, id: i64) {
        let this = *self;
        spawn(async move {
            if api::delete_timing(id).await.is_ok() {
                this.refresh();
            }
        });
    }
}

pub fn use_timings() -> UseTimings {
    let hook = UseTimings {
        items: use_signal(Vec::new),
    };

    use_effect(move || {
        let mut items = hook.items;
        spawn(async move {
            if let Ok(list) = api::list_timings().await {
                items.set(list);
            }
        });
    });

    hook
}
