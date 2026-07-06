# Cycl ハンズオン 02: データ層（Phase 1）

Todoとポモドーロセッションの永続化をRust側に実装します。フロントエンドはこのフェーズの終わりの時点ではまだ何も呼び出しませんが、`cargo test` で全ロジックが単体テストされた状態にします。

> **Tauriの概念: コマンドとState**
> Tauriでは、フロント（JS/TS）からRust関数を呼び出す仕組みを **コマンド** と呼びます。Rust側で `#[tauri::command]` を付けた関数を `invoke_handler` に登録すると、フロントから `invoke("todo_create", { ... })` のように呼べるようになります。
> コマンド間で共有したい状態（今回はDB接続）は `State<T>` として管理し、`app.manage(...)` でTauriに登録します。

## 1. 依存クレートを追加する

`src-tauri/Cargo.toml` の `[dependencies]` に追加します。

```toml
[dependencies]
rusqlite = { version = "0.31", features = ["bundled"] }
thiserror = "1"
```

`features = ["bundled"]` は、実行環境にSQLiteライブラリが入っていなくても動くよう、SQLite本体をRustクレートとして静的リンクする設定です。配布時のトラブルを避けるために指定します。

## 2. ディレクトリ構成

`src-tauri/src/` の下を以下のように分割します。

```
src-tauri/src/
├── main.rs
├── lib.rs
├── error.rs
├── models/
│   ├── mod.rs
│   ├── todo.rs
│   └── session.rs
├── db/
│   ├── mod.rs
│   ├── migrations.rs
│   ├── todo_queries.rs
│   └── session_queries.rs
└── commands/
    ├── mod.rs
    └── todo.rs
```

`db/*_queries.rs` に「Rustの生ロジック」（`&Connection` を受け取るただの関数）を置き、`commands/todo.rs` はそれを `State` から取り出して呼ぶだけの薄いラッパーにします。こうしておくと、後のフェーズでタイマーエンジン（Rustの内部処理）が同じロジックをIPCなしで直接呼び出せます。ポモドーロセッションのテーブルは今回このロジック層とテストだけを作り、Tauriコマンドとしては公開しません。セッションの開始・終了はPhase 3で作るタイマーエンジンが内部で行うので、今の時点でフロントに公開してもYAGNIです。

## 3. エラー型

`src-tauri/src/error.rs`:

```rust
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("todo not found: {0}")]
    TodoNotFound(i64),
}

// Tauriのコマンドがエラーを返す場合、そのエラー型はSerializeを実装している必要があります。
// ここではフロントには文字列化したメッセージだけを渡します。
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

## 4. モデル

`src-tauri/src/models/mod.rs`:

```rust
pub mod session;
pub mod todo;
```

`src-tauri/src/models/todo.rs`:

```rust
use serde::{Deserialize, Serialize};

// フロントエンド(TypeScript)はcamelCaseの慣習に合わせたいので、
// JSONにシリアライズする際のフィールド名だけをcamelCaseに変換する。
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Todo {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
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
}
```

SQLiteに真偽値型はなく `INTEGER` の0/1で表現するため、`from_row` で明示的に変換しています。

`src-tauri/src/models/session.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroSession {
    pub id: i64,
    pub todo_id: i64,
    pub started_at: String,
    pub completed: bool,
}

impl PomodoroSession {
    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(PomodoroSession {
            id: row.get("id")?,
            todo_id: row.get("todo_id")?,
            started_at: row.get("started_at")?,
            completed: row.get::<_, i64>("completed")? != 0,
        })
    }
}
```

## 5. スキーマとマイグレーション

`src-tauri/src/db/migrations.rs`:

```rust
use rusqlite::{Connection, Result};

const MIGRATIONS: &[(&str, &str)] = &[(
    "0001_initial",
    r#"
    CREATE TABLE todo (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        is_completed INTEGER NOT NULL DEFAULT 0,
        pomodoro_count INTEGER NOT NULL DEFAULT 0,
        target_count INTEGER,
        is_active INTEGER NOT NULL DEFAULT 0,
        created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
    );

    CREATE TABLE pomodoro_session (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        todo_id INTEGER NOT NULL REFERENCES todo(id) ON DELETE CASCADE,
        started_at TEXT NOT NULL,
        completed INTEGER NOT NULL DEFAULT 0
    );
    "#,
)];

/// SQLiteの `PRAGMA user_version` を使って、未適用のマイグレーションだけを順番に当てる。
pub fn run(conn: &Connection) -> Result<()> {
    let current_version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let current_version = current_version as usize;

    for (i, (_name, sql)) in MIGRATIONS.iter().enumerate() {
        if i < current_version {
            continue;
        }
        conn.execute_batch(sql)?;
        conn.pragma_update(None, "user_version", (i + 1) as i64)?;
    }

    Ok(())
}
```

将来スキーマを変えるときは、この配列に `("0002_xxx", "ALTER TABLE ...")` を追記するだけで済みます。

`src-tauri/src/db/mod.rs`:

```rust
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub mod migrations;
pub mod session_queries;
pub mod todo_queries;

/// Tauriの `State` として管理する、アプリ全体で共有するデータ。
/// `Arc` で包んでおくことで、Phase 4以降でタイマーエンジンの
/// バックグラウンドタスクにも同じDB接続を安全に共有できる。
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
}

pub fn open(db_path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    migrations::run(&conn)?;
    Ok(conn)
}
```

`Connection` はスレッドをまたいで共有できないため `Mutex` で包みます。今回のような単一ユーザーのデスクトップアプリでは書き込み頻度が低く、コマンドごとに短時間ロックするだけなので、このシンプルな方式で十分です（アクセスが増える場合は `r2d2` 等のコネクションプールへの切り替えを検討します）。

## 6. クエリ関数（Todo）

`src-tauri/src/db/todo_queries.rs`:

```rust
use crate::error::{AppError, AppResult};
use crate::models::todo::Todo;
use rusqlite::Connection;

const SELECT_COLUMNS: &str =
    "id, title, is_completed, pomodoro_count, target_count, is_active, created_at";

pub fn list(conn: &Connection) -> AppResult<Vec<Todo>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_COLUMNS} FROM todo ORDER BY created_at ASC"
    ))?;
    let todos = stmt
        .query_map([], Todo::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(todos)
}

pub fn get(conn: &Connection, id: i64) -> AppResult<Todo> {
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM todo WHERE id = ?1"),
        [id],
        Todo::from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::TodoNotFound(id),
        other => AppError::Database(other),
    })
}

pub fn create(conn: &Connection, title: &str, target_count: Option<i64>) -> AppResult<Todo> {
    conn.execute(
        "INSERT INTO todo (title, target_count) VALUES (?1, ?2)",
        (title, target_count),
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, title: &str, target_count: Option<i64>) -> AppResult<Todo> {
    let affected = conn.execute(
        "UPDATE todo SET title = ?1, target_count = ?2 WHERE id = ?3",
        (title, target_count, id),
    )?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: i64) -> AppResult<()> {
    let affected = conn.execute("DELETE FROM todo WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    Ok(())
}

pub fn toggle_complete(conn: &Connection, id: i64) -> AppResult<Todo> {
    let affected = conn.execute(
        "UPDATE todo SET is_completed = NOT is_completed WHERE id = ?1",
        [id],
    )?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    get(conn, id)
}

/// 「現在取り組むTodo」を切り替える。常に高々1件だけがactiveになるよう、
/// 一度全解除してから指定されたidだけ立てる。
pub fn set_active(conn: &Connection, id: Option<i64>) -> AppResult<()> {
    conn.execute("UPDATE todo SET is_active = 0", [])?;
    if let Some(id) = id {
        let affected = conn.execute("UPDATE todo SET is_active = 1 WHERE id = ?1", [id])?;
        if affected == 0 {
            return Err(AppError::TodoNotFound(id));
        }
    }
    Ok(())
}

/// ポモドーロセッション完了時にタイマーエンジンから呼ばれる。
pub fn increment_pomodoro_count(conn: &Connection, id: i64) -> AppResult<Todo> {
    let affected = conn.execute(
        "UPDATE todo SET pomodoro_count = pomodoro_count + 1 WHERE id = ?1",
        [id],
    )?;
    if affected == 0 {
        return Err(AppError::TodoNotFound(id));
    }
    get(conn, id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::migrations;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn create_and_get_todo() {
        let conn = setup_conn();
        let todo = create(&conn, "牛乳を買う", Some(4)).unwrap();
        assert_eq!(todo.title, "牛乳を買う");
        assert_eq!(todo.target_count, Some(4));
        assert!(!todo.is_completed);

        let fetched = get(&conn, todo.id).unwrap();
        assert_eq!(fetched.id, todo.id);
    }

    #[test]
    fn get_missing_todo_returns_not_found() {
        let conn = setup_conn();
        let err = get(&conn, 999).unwrap_err();
        assert!(matches!(err, AppError::TodoNotFound(999)));
    }

    #[test]
    fn toggle_complete_flips_state() {
        let conn = setup_conn();
        let todo = create(&conn, "掃除", None).unwrap();
        let toggled = toggle_complete(&conn, todo.id).unwrap();
        assert!(toggled.is_completed);
        let toggled_again = toggle_complete(&conn, todo.id).unwrap();
        assert!(!toggled_again.is_completed);
    }

    #[test]
    fn set_active_only_allows_one_at_a_time() {
        let conn = setup_conn();
        let a = create(&conn, "A", None).unwrap();
        let b = create(&conn, "B", None).unwrap();

        set_active(&conn, Some(a.id)).unwrap();
        set_active(&conn, Some(b.id)).unwrap();

        let a_after = get(&conn, a.id).unwrap();
        let b_after = get(&conn, b.id).unwrap();
        assert!(!a_after.is_active);
        assert!(b_after.is_active);
    }

    #[test]
    fn increment_pomodoro_count_increases_by_one() {
        let conn = setup_conn();
        let todo = create(&conn, "作業", None).unwrap();
        let updated = increment_pomodoro_count(&conn, todo.id).unwrap();
        assert_eq!(updated.pomodoro_count, 1);
    }
}
```

## 7. クエリ関数（PomodoroSession）

`src-tauri/src/db/session_queries.rs`:

```rust
use crate::error::AppResult;
use crate::models::session::PomodoroSession;
use rusqlite::Connection;

pub fn create(conn: &Connection, todo_id: i64, started_at: &str) -> AppResult<PomodoroSession> {
    conn.execute(
        "INSERT INTO pomodoro_session (todo_id, started_at) VALUES (?1, ?2)",
        (todo_id, started_at),
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn complete(conn: &Connection, id: i64) -> AppResult<PomodoroSession> {
    conn.execute(
        "UPDATE pomodoro_session SET completed = 1 WHERE id = ?1",
        [id],
    )?;
    get(conn, id)
}

pub fn list_by_todo(conn: &Connection, todo_id: i64) -> AppResult<Vec<PomodoroSession>> {
    let mut stmt = conn.prepare(
        "SELECT id, todo_id, started_at, completed FROM pomodoro_session
         WHERE todo_id = ?1 ORDER BY started_at ASC",
    )?;
    let sessions = stmt
        .query_map([todo_id], PomodoroSession::from_row)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(sessions)
}

fn get(conn: &Connection, id: i64) -> AppResult<PomodoroSession> {
    Ok(conn.query_row(
        "SELECT id, todo_id, started_at, completed FROM pomodoro_session WHERE id = ?1",
        [id],
        PomodoroSession::from_row,
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{migrations, todo_queries};

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        migrations::run(&conn).unwrap();
        conn
    }

    #[test]
    fn create_and_complete_session() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "作業", None).unwrap();

        let session = create(&conn, todo.id, "2026-07-04T10:00:00.000Z").unwrap();
        assert!(!session.completed);

        let completed = complete(&conn, session.id).unwrap();
        assert!(completed.completed);
    }

    #[test]
    fn list_by_todo_returns_only_matching_sessions() {
        let conn = setup_conn();
        let todo_a = todo_queries::create(&conn, "A", None).unwrap();
        let todo_b = todo_queries::create(&conn, "B", None).unwrap();

        create(&conn, todo_a.id, "2026-07-04T10:00:00.000Z").unwrap();
        create(&conn, todo_a.id, "2026-07-04T10:30:00.000Z").unwrap();
        create(&conn, todo_b.id, "2026-07-04T11:00:00.000Z").unwrap();

        let sessions = list_by_todo(&conn, todo_a.id).unwrap();
        assert_eq!(sessions.len(), 2);
    }
}
```

## 8. Tauriコマンド

`src-tauri/src/commands/mod.rs`:

```rust
pub mod todo;
```

`src-tauri/src/commands/todo.rs`:

```rust
use crate::db::{todo_queries, AppState};
use crate::error::AppResult;
use crate::models::todo::Todo;
use tauri::State;

#[tauri::command]
pub fn todo_list(state: State<AppState>) -> AppResult<Vec<Todo>> {
    let conn = state.db.lock().unwrap();
    todo_queries::list(&conn)
}

#[tauri::command]
pub fn todo_create(
    state: State<AppState>,
    title: String,
    target_count: Option<i64>,
) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    todo_queries::create(&conn, &title, target_count)
}

#[tauri::command]
pub fn todo_update(
    state: State<AppState>,
    id: i64,
    title: String,
    target_count: Option<i64>,
) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    todo_queries::update(&conn, id, &title, target_count)
}

#[tauri::command]
pub fn todo_delete(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    todo_queries::delete(&conn, id)
}

#[tauri::command]
pub fn todo_toggle_complete(state: State<AppState>, id: i64) -> AppResult<Todo> {
    let conn = state.db.lock().unwrap();
    todo_queries::toggle_complete(&conn, id)
}

#[tauri::command]
pub fn todo_set_active(state: State<AppState>, id: Option<i64>) -> AppResult<()> {
    let conn = state.db.lock().unwrap();
    todo_queries::set_active(&conn, id)
}
```

`State<AppState>` を引数に取ると、Tauriが呼び出し時に自動で管理下のインスタンスを渡してくれます（DIのようなもの）。

## 9. `lib.rs` に組み込む

`src-tauri/src/lib.rs` 全体:

```rust
#![warn(clippy::all)]

mod commands;
mod db;
mod error;
mod models;

use db::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("cycl.sqlite3");
            let conn = db::open(&db_path)?;
            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::todo::todo_list,
            commands::todo::todo_create,
            commands::todo::todo_update,
            commands::todo::todo_delete,
            commands::todo::todo_toggle_complete,
            commands::todo::todo_set_active,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

`setup` クロージャの戻り値は `Result<(), Box<dyn std::error::Error>>` なので、`?` で `io::Error` や `rusqlite::Error` をそのまま伝播できます。DBファイルはOSごとのアプリデータディレクトリ（macOSでは `~/Library/Application Support/com.cycl.app/`）に作成されます。

## 10. テストを実行する

```bash
cargo test --workspace
```

すべてのテストが緑になることを確認してください。

## 11. CIにRustテストを追加する

`.github/workflows/ci.yml` の `rust-lint` ジョブに1ステップ追加します(フロントも含めたワークスペース全体が対象です)。

```yaml
  rust-lint:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - run: cargo test --workspace
```

## 12. コミットする

```bash
git add .
git commit -m "feat: add sqlite-backed todo and session data layer"
```

## OSSチェックポイント

- [ ] `cargo test --workspace` が全て通る
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` が警告なしで通る
- [ ] フロントに公開していないモジュール（session）が、将来利用する場所（Phase 3のタイマーエンジン）を意識した設計になっているか再確認した
- [ ] CIが更新後も緑になることを確認した

次は [03-todo-ui.md](03-todo-ui.md) で、このコマンド群を使うTodo UIを作ります。
