use dioxus::prelude::*;
use shared::Tag;

use crate::tauri_api::tag as api;

#[derive(Clone, Copy)]
pub struct UseTags {
    pub items: Signal<Vec<Tag>>,
}

impl UseTags {
    pub fn refresh(&self) {
        let mut items = self.items;
        spawn(async move {
            if let Ok(list) = api::list_tags().await {
                items.set(list);
            }
        });
    }

    pub fn add(&self, name: String, color: String) {
        let this = *self;
        spawn(async move {
            if api::create_tag(&name, &color).await.is_ok() {
                this.refresh();
            }
        });
    }

    pub fn remove(&self, id: i64) {
        let this = *self;
        spawn(async move {
            if api::delete_tag(id).await.is_ok() {
                this.refresh();
            }
        });
    }
}

pub fn use_tags() -> UseTags {
    let hook = UseTags {
        items: use_signal(Vec::new),
    };

    use_effect(move || {
        let mut items = hook.items;
        spawn(async move {
            if let Ok(list) = api::list_tags().await {
                items.set(list);
            }
        });
    });

    hook
}
