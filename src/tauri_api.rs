use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    // ときにpanicする。AppErrorは文字列にシリアライズされるため、
    // 失敗時はJSの文字列がErrとして返ってくる。
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn invoke_raw(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn listen_raw(
        event: &str,
        handler: &Closure<dyn FnMut(JsValue)>,
    ) -> Result<JsValue, JsValue>;
}

async fn invoke_inner<T: DeserializeOwned>(cmd: &str, args: JsValue) -> Result<T, String> {
    let result = invoke_raw(cmd, args)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

pub async fn invoke0<T: DeserializeOwned>(cmd: &str) -> Result<T, String> {
    invoke_inner(cmd, JsValue::NULL).await
}

pub async fn invoke<A: Serialize, T: DeserializeOwned>(cmd: &str, args: &A) -> Result<T, String> {
    let args = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    invoke_inner(cmd, args).await
}

/// `event_name` を購読し、届いたペイロードを`on_payload`に渡し続ける。
/// アプリのライフタイム全体で購読し続ける前提のシングルトン用途向け。
pub fn listen<T: DeserializeOwned + 'static>(
    event_name: &'static str,
    mut on_payload: impl FnMut(T) + 'static,
) {
    let closure = Closure::wrap(Box::new(move |event: JsValue| {
        let payload = js_sys::Reflect::get(&event, &JsValue::from_str("payload")).unwrap();
        if let Ok(value) = serde_wasm_bindgen::from_value::<T>(payload) {
            on_payload(value);
        }
    }) as Box<dyn FnMut(JsValue)>);

    spawn_local(async move {
        let _ = listen_raw(event_name, &closure).await;
        closure.forget();
    });
}

pub mod timer {
    use super::invoke0;
    use shared::TimerState;

    pub async fn get_timer_state() -> Result<TimerState, String> {
        invoke0("timer_get_state").await
    }

    pub async fn start_timer() -> Result<TimerState, String> {
        invoke0("timer_start").await
    }

    pub async fn pause_timer() -> Result<TimerState, String> {
        invoke0("timer_pause").await
    }

    pub async fn reset_timer() -> Result<TimerState, String> {
        invoke0("timer_reset").await
    }
}

pub mod timing {
    use super::{invoke, invoke0};
    use serde::Serialize;
    use shared::Timing;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CreateArgs<'a> {
        name: &'a str,
        color: &'a str,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct IdArgs {
        id: i64,
    }

    pub async fn list_timings() -> Result<Vec<Timing>, String> {
        invoke0("timing_list").await
    }

    pub async fn create_timing(name: &str, color: &str) -> Result<Timing, String> {
        invoke("timing_create", &CreateArgs { name, color }).await
    }

    pub async fn delete_timing(id: i64) -> Result<(), String> {
        invoke("timing_delete", &IdArgs { id }).await
    }
}

pub mod tag {
    use super::invoke;
    use super::invoke0;
    use serde::Serialize;
    use shared::{Tag, Todo};

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CreateArgs<'a> {
        name: &'a str,
        color: &'a str,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct IdArgs {
        id: i64,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TodoTagArgs {
        todo_id: i64,
        tag_id: i64,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct TagIdArgs {
        tag_id: i64,
    }

    pub async fn list_tags() -> Result<Vec<Tag>, String> {
        invoke0("tag_list").await
    }

    pub async fn create_tag(name: &str, color: &str) -> Result<Tag, String> {
        invoke("tag_create", &CreateArgs { name, color }).await
    }

    pub async fn delete_tag(id: i64) -> Result<(), String> {
        invoke("tag_delete", &IdArgs { id }).await
    }

    pub async fn add_tag(todo_id: i64, tag_id: i64) -> Result<Todo, String> {
        invoke("todo_add_tag", &TodoTagArgs { todo_id, tag_id }).await
    }

    pub async fn remove_tag(todo_id: i64, tag_id: i64) -> Result<Todo, String> {
        invoke("todo_remove_tag", &TodoTagArgs { todo_id, tag_id }).await
    }

    pub async fn list_todos_by_tag(tag_id: i64) -> Result<Vec<Todo>, String> {
        invoke("todo_list_by_tag", &TagIdArgs { tag_id }).await
    }
}

pub mod todo {
    use super::{invoke, invoke0};
    use serde::Serialize;
    use shared::Todo;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CreateArgs<'a> {
        title: &'a str,
        target_count: Option<i64>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct UpdateArgs<'a> {
        id: i64,
        title: &'a str,
        target_count: Option<i64>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct IdArgs {
        id: Option<i64>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ReorderArgs {
        ordered_ids: Vec<i64>,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct CategoryArgs {
        category: String,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct UpdateCategoryArgs {
        id: i64,
        category: String,
    }

    pub async fn list_todos() -> Result<Vec<Todo>, String> {
        invoke0("todo_list").await
    }

    pub async fn create_todo(title: &str, target_count: Option<i64>) -> Result<Todo, String> {
        invoke(
            "todo_create",
            &CreateArgs {
                title,
                target_count,
            },
        )
        .await
    }

    pub async fn update_todo(
        id: i64,
        title: &str,
        target_count: Option<i64>,
    ) -> Result<Todo, String> {
        invoke(
            "todo_update",
            &UpdateArgs {
                id,
                title,
                target_count,
            },
        )
        .await
    }

    pub async fn delete_todo(id: i64) -> Result<(), String> {
        invoke("todo_delete", &IdArgs { id: Some(id) }).await
    }

    pub async fn toggle_complete(id: i64) -> Result<Todo, String> {
        invoke("todo_toggle_complete", &IdArgs { id: Some(id) }).await
    }

    pub async fn set_active(id: Option<i64>) -> Result<(), String> {
        invoke("todo_set_active", &IdArgs { id }).await
    }

    pub async fn reorder_todos(ordered_ids: Vec<i64>) -> Result<Vec<Todo>, String> {
        invoke("todo_reorder", &ReorderArgs { ordered_ids }).await
    }

    pub async fn list_todos_by_category(category: &str) -> Result<Vec<Todo>, String> {
        invoke(
            "todo_list_by_category",
            &CategoryArgs {
                category: category.to_string(),
            },
        )
        .await
    }

    pub async fn update_category(id: i64, category: &str) -> Result<Todo, String> {
        invoke(
            "todo_update_category",
            &UpdateCategoryArgs {
                id,
                category: category.to_string(),
            },
        )
        .await
    }
}
