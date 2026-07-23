use dioxus::prelude::*;
use shared::Todo;

use crate::tauri_api::todo as api;

#[derive(Clone, Copy)]
pub struct UseTodos {
    pub items: Signal<Vec<Todo>>,
    pub is_loading: Signal<bool>,
}

impl UseTodos {
    pub fn refresh(&self) {
        let mut items = self.items;
        spawn(async move {
            if let Ok(list) = api::list_todos().await {
                items.set(list);
            }
        });
    }

    pub fn add(&self, title: String, target_count: Option<i64>) {
        let this = *self;
        spawn(async move {
            if api::create_todo(&title, target_count).await.is_ok() {
                this.refresh();
            }
        });
    }

    pub fn toggle_complete(&self, id: i64) {
        let this = *self;
        spawn(async move {
            if api::toggle_complete(id).await.is_ok() {
                this.refresh();
            }
        });
    }

    pub fn remove(&self, id: i64) {
        let this = *self;
        spawn(async move {
            if api::delete_todo(id).await.is_ok() {
                this.refresh();
            }
        });
    }

    pub fn select_active(&self, id: i64) {
        let this = *self;
        spawn(async move {
            if api::set_active(Some(id)).await.is_ok() {
                this.refresh();
            }
        });
    }

    pub fn reorder(&self, ordered_ids: Vec<i64>) {
        let this = *self;
        spawn(async move {
            if api::reorder_todos(ordered_ids).await.is_ok() {
                this.refresh();
            }
        });
    }
}

pub fn use_todos() -> UseTodos {
    let hook = UseTodos {
        items: use_signal(Vec::new),
        is_loading: use_signal(|| true),
    };

    use_effect(move || {
        let mut items = hook.items;
        let mut is_loading = hook.is_loading;
        spawn(async move {
            if let Ok(list) = api::list_todos().await {
                items.set(list);
            }
            is_loading.set(false);
        });

        // Rust側でセッションが完了しTodoが更新されたら、一覧を取り直す。
        crate::tauri_api::listen::<()>("todos:changed", move |_| {
            hook.refresh();
        });
    });

    hook
}
