# Cycl ハンズオン 05: Todo×タイマー連携（Phase 4)

Phase 2で作った「現在取り組むTodo」と、Phase 3で作ったタイマーをつなげます。作業セッションが完了したら、Rust側だけで「セッション記録 → Todoのポモドーロ数を更新 → macOS通知」を完結させます。フロントは何もロジックを持ちません。

## 1. 依存クレートを追加する

`src-tauri/Cargo.toml`:

```toml
[dependencies]
chrono = { version = "0.4", features = ["clock"] }
tauri-plugin-notification = "2"
```

`chrono` はセッション開始時刻をISO8601形式で記録するために、`tauri-plugin-notification` はRustからmacOS通知を発火するために使います。

## 2. 通知の権限を設定する

Tauri v2ではプラグインが提供するコマンドの利用にcapabilities(権限)の許可が必要です。`src-tauri/capabilities/default.json` の `permissions` に追記します。

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "opener:default", "notification:default"]
}
```

## 3. `todo_queries` に「現在アクティブなTodo取得」を追加する

`src-tauri/src/db/todo_queries.rs` に追記します(`Todo::from_row`ではなく、Phase 2で`shared`クレートへの移動に合わせて用意した自由関数`todo_from_row`を使います)。

```rust
pub fn get_active(conn: &Connection) -> AppResult<Option<Todo>> {
    conn.query_row(
        &format!("SELECT {SELECT_COLUMNS} FROM todo WHERE is_active = 1"),
        [],
        todo_from_row,
    )
    .map(Some)
    .or_else(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(AppError::Database(other)),
    })
}
```

`#[cfg(test)] mod tests` の中にテストを追加します。

```rust
    #[test]
    fn get_active_returns_the_active_todo() {
        let conn = setup_conn();
        let a = create(&conn, "A", None).unwrap();
        let _b = create(&conn, "B", None).unwrap();

        assert!(get_active(&conn).unwrap().is_none());

        set_active(&conn, Some(a.id)).unwrap();
        let active = get_active(&conn).unwrap().unwrap();
        assert_eq!(active.id, a.id);
    }
```

## 4. `session_queries` に「完了済みセッションとして記録」を追加する

作業セッションはタイマーが最後まで進んだ時点でまとめて記録するので、開始と完了を別々に呼ぶのではなく1回で完了済みレコードを作る関数を用意します。

`src-tauri/src/db/session_queries.rs` に追記します。

```rust
pub fn record_completed(
    conn: &Connection,
    todo_id: i64,
    started_at: &str,
) -> AppResult<PomodoroSession> {
    conn.execute(
        "INSERT INTO pomodoro_session (todo_id, started_at, completed) VALUES (?1, ?2, 1)",
        (todo_id, started_at),
    )?;
    get(conn, conn.last_insert_rowid())
}
```

テストも追加します。

```rust
    #[test]
    fn record_completed_inserts_an_already_completed_session() {
        let conn = setup_conn();
        let todo = todo_queries::create(&conn, "作業", None).unwrap();
        let session = record_completed(&conn, todo.id, "2026-07-04T10:00:00+09:00").unwrap();
        assert!(session.completed);
    }
```

## 5. タイマーエンジンを拡張する

`src-tauri/src/timer/engine.rs` を書き換えます。作業セッションの開始時刻を記録しておき、作業フェーズが完了した瞬間にDB更新と通知をまとめて行います。

```rust
use crate::db::{session_queries, todo_queries};
use chrono::Utc;
use rusqlite::Connection;
use shared::{TimerPhase, TimerSettings, TimerState};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::time::{interval, Duration};

pub struct TimerEngine {
    state: Arc<Mutex<TimerState>>,
    session_started_at: Arc<Mutex<Option<String>>>,
}

impl TimerEngine {
    pub fn new(app_handle: AppHandle, db: Arc<Mutex<Connection>>) -> Self {
        let state = Arc::new(Mutex::new(TimerState::new(TimerSettings::default())));
        let session_started_at = Arc::new(Mutex::new(None));
        spawn_tick_loop(app_handle, state.clone(), db, session_started_at.clone());
        TimerEngine {
            state,
            session_started_at,
        }
    }

    pub fn snapshot(&self) -> TimerState {
        self.state.lock().unwrap().clone()
    }

    pub fn start(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        if state.phase == TimerPhase::Work && !state.is_running {
            let mut started_at = self.session_started_at.lock().unwrap();
            if started_at.is_none() {
                *started_at = Some(Utc::now().to_rfc3339());
            }
        }
        state.is_running = true;
        state.clone()
    }

    pub fn pause(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        state.is_running = false;
        state.clone()
    }

    pub fn reset(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
        state.reset_current_phase();
        *self.session_started_at.lock().unwrap() = None;
        state.clone()
    }
}

fn spawn_tick_loop(
    app_handle: AppHandle,
    state: Arc<Mutex<TimerState>>,
    db: Arc<Mutex<Connection>>,
    session_started_at: Arc<Mutex<Option<String>>>,
) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;

            let mut just_completed_work = false;
            let snapshot = {
                let mut state = state.lock().unwrap();
                if state.is_running {
                    if state.remaining_secs > 0 {
                        state.remaining_secs -= 1;
                    }
                    if state.remaining_secs == 0 {
                        just_completed_work = state.phase == TimerPhase::Work;
                        state.advance_phase();
                    }
                }
                state.clone()
            };

            if just_completed_work {
                if let Some(started_at) = session_started_at.lock().unwrap().take() {
                    record_work_session(&app_handle, &db, &started_at);
                }
            }

            let _ = app_handle.emit("timer:tick", &snapshot);
        }
    });
}

/// 作業セッションが1本終わった瞬間の後処理。
/// アクティブなTodoが選ばれていなければ何もしない(記録も通知もしない)。
fn record_work_session(app_handle: &AppHandle, db: &Arc<Mutex<Connection>>, started_at: &str) {
    let conn = db.lock().unwrap();

    let Ok(Some(todo)) = todo_queries::get_active(&conn) else {
        return;
    };
    if session_queries::record_completed(&conn, todo.id, started_at).is_err() {
        return;
    }
    let Ok(updated) = todo_queries::increment_pomodoro_count(&conn, todo.id) else {
        return;
    };

    drop(conn); // 通知呼び出しの前にロックを解放する

    let _ = app_handle.emit("todos:changed", ());

    let _ = app_handle
        .notification()
        .builder()
        .title("ポモドーロ完了")
        .body(format!(
            "「{}」を1セット完了しました 🍅×{}",
            updated.title, updated.pomodoro_count
        ))
        .show();
}
```

`just_completed_work` で「今回のtickで作業フェーズが終わったか」を判定し、終わっていた場合だけDB更新と通知を行います。休憩フェーズの完了では何もしません。

## 6. `lib.rs` を更新する

DB接続をTodoコマンドとタイマーエンジンの両方で共有するように変更し、通知プラグインを登録します。

```rust
#![warn(clippy::all)]

mod commands;
mod db;
mod error;
mod models;
mod timer;

use db::AppState;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use timer::engine::TimerEngine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("cycl.sqlite3");
            let conn = db::open(&db_path)?;
            let db = Arc::new(Mutex::new(conn));

            app.manage(AppState { db: db.clone() });
            app.manage(TimerEngine::new(app.handle().clone(), db.clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::todo::todo_list,
            commands::todo::todo_create,
            commands::todo::todo_update,
            commands::todo::todo_delete,
            commands::todo::todo_toggle_complete,
            commands::todo::todo_set_active,
            commands::timer::timer_get_state,
            commands::timer::timer_start,
            commands::timer::timer_pause,
            commands::timer::timer_reset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## 7. フロントでTodo一覧の自動更新を購読する

セッション完了はバックグラウンドのRustタスクの中で起きるため、フロントは「いつ完了したか」を知りません。Rustが発行する `todos:changed` イベントを、Phase 3で作った`tauri_api::listen`で購読して、届いたら一覧を取り直します。

`src/hooks/use_todos.rs` の `impl UseTodos` にある `fn refresh` を `pub fn refresh` に変更します(Phase 2ではモジュール内だけで使う非公開メソッドでしたが、イベントハンドラからも呼べるようにします)。

```rust
impl UseTodos {
    pub fn refresh(&self) {
        let mut items = self.items;
        spawn(async move {
            if let Ok(list) = api::list_todos().await {
                items.set(list);
            }
        });
    }

    // add / toggle_complete / remove / select_active は変更なし
}
```

`use_todos()` 関数に、購読を追記します。

```rust
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
```

(`add`/`toggle_complete`/`remove`/`select_active`など他のメソッドはPhase 2のままで構いません。)

## 8. 動作確認

1. `cargo tauri dev` を起動する
2. Todoを1つ作り、タイトルをクリックして「現在取り組むTodo」に選択する(枠がハイライトされる)
3. 確認を早くするため、一時的に `shared/src/timer.rs` の `TimerSettings::default()` の `work_minutes` を `1`(1分)に変更してビルドし直す
4. タイマーの「開始」を押し、1分待つ
5. macOS通知が表示され、選択していたTodoの🍅カウントが自動で増えることを確認する
6. 確認できたら `work_minutes` を `25` に戻す

初回はmacOSが通知の許可を求めるダイアログを出すことがあります。許可しないと通知は表示されません(システム設定 > 通知 > Cycl から後で変更できます)。

## 9. コミットする

```bash
git add .
git commit -m "feat: record pomodoro sessions and notify on work session completion"
```

## OSSチェックポイント

- [ ] `cargo test --workspace` が通る(`get_active`, `record_completed`のテスト含む)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` が警告なしで通る
- [ ] Todo未選択のまま作業セッションを完了させても、エラーにならず何も記録されないことを確認した
- [ ] 通知とカウント更新を実アプリで確認した

次は [06-tray.md](06-tray.md) で、メニューバー常駐を実装します。
