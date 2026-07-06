# Cycl ハンズオン 03: Todoリスト UI（Phase 2）

Phase 1で作ったTauriコマンドを呼び出す、Todo一覧・追加・完了切替・削除・「現在取り組むTodo」選択のUIを作ります。ロジック（バリデーション以外）はすべてRust側にあるため、フロントはコマンドを呼んで結果を描画するだけです。

## 1. `Todo` 構造体を `shared` クレートに移す

`src-tauri/src/models/todo.rs` にあった`Todo`をフロントからも使えるように`shared`クレートへ移動します。React版では同じ形の型をTypeScript側に手書きで複製していましたが（`src/lib/types.ts`）、フロントもRustになったので**同じ構造体をそのまま共有**できます。

`shared/src/todo.rs`（新規、`models/todo.rs`の内容を移動）:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub is_completed: bool,
    pub pomodoro_count: i64,
    pub target_count: Option<i64>,
    pub is_active: bool,
    pub created_at: String,
}
```

`PartialEq`を追加しています。DioxusのコンポーネントProps(後述)は差分検知のため`PartialEq`を要求するので、フロントで使う共有型には基本的に付けておきます。

`from_row`（`rusqlite::Row`からの変換）は**移動しません**。Rustの orphan rule により、他クレート(`shared`)で定義した型に対して`src-tauri`側から新しい inherent メソッドを生やすことはできないためです。代わりに`src-tauri`側にただの関数として残します(次のステップで対応)。

`shared/src/lib.rs`:

```rust
mod todo;

pub use todo::Todo;
```

`src-tauri/src/models/todo.rs` は削除し、`src-tauri/src/models/mod.rs` から `pub mod todo;` を削除します(`session`はフロントに公開しないため`shared`には移さず、今まで通り`src-tauri/src/models/session.rs`に残します)。

## 2. `src-tauri` 側の参照を更新する

`src-tauri/Cargo.toml` に(01ハンズオンで追加済みのはずですが)`shared`への依存があることを確認します。

```toml
[dependencies]
shared = { path = "../shared" }
```

`src-tauri/src/db/todo_queries.rs` の先頭を書き換え、`Todo::from_row`だったところを自由関数`todo_from_row`に変更します。

```rust
use crate::error::{AppError, AppResult};
use rusqlite::Connection;
use shared::Todo;

const SELECT_COLUMNS: &str =
    "id, title, is_completed, pomodoro_count, target_count, is_active, created_at";

fn todo_from_row(row: &rusqlite::Row) -> rusqlite::Result<Todo> {
    Ok(Todo {
        id: row.get("id")?,
        title: row.get("title")?,
        is_completed: row.get::<_, i64>("is_completed")? != 0,
        pomodoro_count: row.get("pomodoro_count")?,
        target_count: row.get("target_count")?,
        is_active: row.get::<_, i64>("is_active")? != 0,
        created_at: row.get("created_at")?,
    })
}
```

ファイル内の`Todo::from_row`を呼んでいた箇所(`list`/`get`など)はすべて`todo_from_row`に置き換えます。ロジック自体は変わらないため、Phase 1で書いたテストはそのまま通ります。

`src-tauri/src/commands/todo.rs` の `use crate::models::todo::Todo;` は `use shared::Todo;` に変更します。

## 3. wasm-bindgen経由のIPCラッパーを作る

> **概念: フロント(wasm)からのIPC呼び出し**
> ReactではNPMパッケージ`@tauri-apps/api`の`invoke()`をそのままimportして呼べましたが、Dioxus(wasm)からは同じNPMパッケージをimportすることはできません。Tauriは`tauri.conf.json`で`"app": { "withGlobalTauri": true }`を設定すると、WebView側に`window.__TAURI__`というグローバルオブジェクトを注入してくれます。これを`wasm-bindgen`の`extern`ブロックで宣言し、Rust(wasm)側から直接呼び出します。

`src/tauri_api.rs`(新規):

```rust
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // `catch` を付けないと、Rustコマンドがエラーを返した(=JS Promiseがrejectされた)
    // ときにpanicする。AppErrorは文字列にシリアライズされるため、
    // 失敗時はJSの文字列がErrとして返ってくる。
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn invoke_raw(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

async fn invoke_inner<T: DeserializeOwned>(cmd: &str, args: JsValue) -> Result<T, String> {
    let result = invoke_raw(cmd, args)
        .await
        .map_err(|e| e.as_string().unwrap_or_else(|| format!("{e:?}")))?;
    serde_wasm_bindgen::from_value(result).map_err(|e| e.to_string())
}

/// 引数を取らないコマンド用。
pub async fn invoke0<T: DeserializeOwned>(cmd: &str) -> Result<T, String> {
    invoke_inner(cmd, JsValue::NULL).await
}

/// 引数を取るコマンド用。`args`はTauri側のパラメータ名に合わせて
/// `#[serde(rename_all = "camelCase")]` を付けた構造体を渡す。
pub async fn invoke<A: Serialize, T: DeserializeOwned>(cmd: &str, args: &A) -> Result<T, String> {
    let args = serde_wasm_bindgen::to_value(args).map_err(|e| e.to_string())?;
    invoke_inner(cmd, args).await
}
```

`src/tauri_api.rs` に続けて、Todo用の型付きAPI関数を書きます(旧`src/lib/api.ts`相当)。

```rust
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

    pub async fn list_todos() -> Result<Vec<Todo>, String> {
        invoke0("todo_list").await
    }

    pub async fn create_todo(title: &str, target_count: Option<i64>) -> Result<Todo, String> {
        invoke("todo_create", &CreateArgs { title, target_count }).await
    }

    pub async fn update_todo(id: i64, title: &str, target_count: Option<i64>) -> Result<Todo, String> {
        invoke("todo_update", &UpdateArgs { id, title, target_count }).await
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
}
```

`src/main.rs` に `mod tauri_api;` を追加します。

## 4. カスタムフックで状態管理する(`use_todos`)

Dioxusには`useState`/`useEffect`/`useCallback`の代わりに`use_signal`/`use_effect`/`spawn`があります。`Signal<T>`は`Copy`なので、非同期タスク(`spawn`)の中にそのまま持ち込めます。

`src/hooks/use_todos.rs`(新規):

```rust
use dioxus::prelude::*;
use shared::Todo;

use crate::tauri_api::todo as api;

#[derive(Clone, Copy)]
pub struct UseTodos {
    pub items: Signal<Vec<Todo>>,
    pub is_loading: Signal<bool>,
}

impl UseTodos {
    fn refresh(&self) {
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
    });

    hook
}
```

`use_effect`はデフォルトで初回レンダー後に1回だけ実行されます(中でsignalを読んでいないため、依存なし=マウント時1回、と解釈されます)。サーバー(DB)の状態が唯一の真実の情報源なので、更新系の操作は必ず「Rustに反映 → 一覧を取り直す」という流れにし、フロント側では楽観的更新をしません。

## 5. コンポーネントを作る

`src/components/mod.rs`:

```rust
mod todo_form;
mod todo_item;
mod todo_list;

pub use todo_form::TodoForm;
pub use todo_item::TodoItem;
pub use todo_list::TodoList;
```

`src/components/todo_form.rs`:

```rust
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
                placeholder: "新しいTodo",
                aria_label: "Todoのタイトル",
                oninput: move |e| title.set(e.value()),
            }
            input {
                value: "{target_count}",
                r#type: "number",
                min: "0",
                placeholder: "目標🍅数",
                aria_label: "目標ポモドーロ数",
                oninput: move |e| target_count.set(e.value()),
            }
            button { r#type: "submit", "追加" }
        }
    }
}
```

`src/components/todo_item.rs`:

```rust
use dioxus::prelude::*;
use shared::Todo;

#[component]
pub fn TodoItem(
    todo: Todo,
    on_toggle_complete: EventHandler<i64>,
    on_select_active: EventHandler<i64>,
    on_delete: EventHandler<i64>,
) -> Element {
    let target_label = todo
        .target_count
        .map(|target| format!(" / {target}"))
        .unwrap_or_default();
    let id = todo.id;

    rsx! {
        li {
            class: if todo.is_active { "todo-item todo-item--active" } else { "todo-item" },
            input {
                r#type: "checkbox",
                checked: todo.is_completed,
                aria_label: "{todo.title}を完了にする",
                onchange: move |_| on_toggle_complete.call(id),
            }
            button {
                class: if todo.is_completed { "todo-item__title todo-item__title--done" } else { "todo-item__title" },
                onclick: move |_| on_select_active.call(id),
                "{todo.title}"
            }
            span { class: "todo-item__count", "🍅×{todo.pomodoro_count}{target_label}" }
            button { class: "todo-item__delete", onclick: move |_| on_delete.call(id), "削除" }
        }
    }
}
```

`src/components/todo_list.rs`:

```rust
use dioxus::prelude::*;

use super::{TodoForm, TodoItem};
use crate::hooks::use_todos::use_todos;

#[component]
pub fn TodoList() -> Element {
    let todos = use_todos();

    if *todos.is_loading.read() {
        return rsx! { p { class: "muted", "読み込み中..." } };
    }

    rsx! {
        div { class: "todo-list",
            TodoForm {
                on_submit: move |(title, target_count): (String, Option<i64>)| {
                    todos.add(title, target_count);
                }
            }
            ul {
                for todo in todos.items.read().iter().cloned() {
                    TodoItem {
                        key: "{todo.id}",
                        todo: todo.clone(),
                        on_toggle_complete: move |id| todos.toggle_complete(id),
                        on_select_active: move |id| todos.select_active(id),
                        on_delete: move |id| todos.remove(id),
                    }
                }
            }
        }
    }
}
```

`src/app.rs`(生成直後のgreetサンプルを置き換え):

```rust
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
```

`src/main.rs`:

```rust
#![warn(clippy::all)]

mod app;
mod components;
mod hooks;
mod tauri_api;

use app::App;
use dioxus::prelude::*;
use dioxus_logger::tracing::Level;

fn main() {
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    launch(App);
}
```

`src/hooks/mod.rs`:

```rust
pub mod use_todos;
```

## 6. CSSを追加する

`assets/styles.css` に追記します。

```css
.app {
  display: flex;
  justify-content: center;
  padding: 2rem;
}

.todo-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 28rem;
}

.todo-form {
  display: flex;
  gap: 0.5rem;
}

.todo-form input {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.4rem 0.6rem;
  background: transparent;
  color: var(--foreground);
}

.todo-item {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
}

.todo-item--active {
  border-color: var(--primary);
  background: color-mix(in srgb, var(--primary) 12%, transparent);
}

.todo-item__title {
  flex: 1;
  text-align: left;
  background: none;
  border: none;
  color: inherit;
  cursor: pointer;
}

.todo-item__title--done {
  color: var(--muted-foreground);
  text-decoration: line-through;
}

.todo-item__count {
  font-size: 0.85rem;
  color: var(--muted-foreground);
}

.muted {
  color: var(--muted-foreground);
}
```

## 7. 動作確認

```bash
cargo tauri dev
```

Todoの追加・完了チェック・タイトルクリックによる「現在取り組むTodo」の選択(枠がハイライトされる)・削除ができることを確認してください。アプリを再起動しても内容がSQLiteに保存されているので消えません。

> **フロントの専用UIテストについて**: React版ではvitest + Testing Libraryでフォームのバリデーションやクリック挙動をテストしていましたが、Dioxusはコンポーネントテストのエコシステムがまだ未成熟です。フロントは表示ロジックをほぼ持たない薄いView層であることも踏まえ、Cyclではフロント専用のUIテストは持たず、このOSSチェックポイントでの手動確認に一本化します。ロジック(バリデーション込みでもRust側にあるもの)は引き続き`cargo test`でカバーします。

## 8. コミットする

```bash
git add .
git commit -m "feat: add todo list UI with create, toggle, select-active and delete"
```

## OSSチェックポイント

- [ ] `cargo test --workspace` が全て通る(Phase 1のテストが引き続きグリーン)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` が通る
- [ ] UI操作(追加・完了・選択・削除)を実アプリで一通り試した
- [ ] READMEの機能一覧と実装状況にズレがないか確認した

次は [04-timer.md](04-timer.md) で、Rust側が状態を完全に保有するポモドーロタイマーを実装します。
