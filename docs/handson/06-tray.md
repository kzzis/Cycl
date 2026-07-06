# Cycl ハンズオン 06: メニューバー常駐（Phase 5）

トレイアイコンを常駐させ、タイマー実行中は残り時間をメニューバーに表示し、ウィンドウの表示/非表示をメニューから切り替えられるようにします。ウィンドウを閉じてもアプリ自体は終了せず、常駐し続けます。

## 1. `tray-icon` フィーチャーを有効にする

`src-tauri/Cargo.toml` の `tauri` 依存に `tray-icon` フィーチャーを追加します。

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
```

## 2. `format_mm_ss` を再利用する

メニューバーの残り時間表示にも、Phase 3で`shared`クレートに用意した`format_mm_ss`(`shared/src/timer.rs`)をそのまま使います。フロントのリング表示と全く同じ関数なので、ここで新たに定義する必要はありません。

## 3. トレイアイコンとウィンドウ制御をまとめたモジュールを作る

`src-tauri/src/tray.rs`:

```rust
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let toggle_window = MenuItem::with_id(
        app,
        "toggle_window",
        "ウィンドウを表示/非表示",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_window, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle_window" => toggle_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;

    // ウィンドウの「閉じる」はアプリ終了ではなく非表示にする。
    // 常駐アプリの終了は必ずトレイメニューの「終了」から行う。
    if let Some(window) = app.get_webview_window("main") {
        let window_for_close = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_for_close.hide();
            }
        });
    }

    // Dockにアイコンを出さず、メニューバー常駐アプリらしい見た目にする。
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    Ok(())
}

pub fn toggle_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// トレイアイコンのタイトル(macOSのメニューバーに出る文字列)を更新する。
pub fn update_title(app: &AppHandle, title: Option<String>) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(title);
    }
}
```

## 4. タイマーエンジンからメニューバーの表示を更新する

`src-tauri/src/timer/engine.rs` の `import` と tick ループを変更します。

```rust
use shared::{format_mm_ss, TimerPhase, TimerSettings, TimerState};
```

tickループ内、`app_handle.emit("timer:tick", &snapshot);` の直前に1行追加します。

```rust
            let title = snapshot
                .is_running
                .then(|| format_mm_ss(snapshot.remaining_secs));
            crate::tray::update_title(&app_handle, title);

            let _ = app_handle.emit("timer:tick", &snapshot);
```

タイマーが停止しているときはタイトルを消し、アイコンだけを表示します。

## 5. `lib.rs` にトレイのセットアップを組み込む

```rust
#![warn(clippy::all)]

mod commands;
mod db;
mod error;
mod models;
mod timer;
mod tray;

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
            tray::setup(app.handle())?;
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

## 6. 動作確認

```bash
cargo test --workspace
cargo tauri dev
```

- Dockにアイコンが出ず、メニューバーにCyclのアイコンが表示されることを確認する
- タイマーを開始すると、メニューバーのアイコン横に残り時間(`24:59`のような表示)が出ることを確認する
- アイコンをクリックし、「ウィンドウを表示/非表示」でウィンドウの開閉ができることを確認する
- ウィンドウの「閉じる」ボタン(赤信号)を押してもアプリが終了せず、メニューから再度表示できることを確認する
- トレイメニューの「終了」でアプリが完全に終了することを確認する

## 7. コミットする

```bash
git add .
git commit -m "feat: add menu bar tray icon with remaining time and window toggle"
```

## OSSチェックポイント

- [ ] `cargo test --workspace` が通る
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` が警告なしで通る
- [ ] ウィンドウを閉じてもアプリが常駐し続けることを確認した
- [ ] トレイメニューからの終了で確実にプロセスが終わることを確認した

次は [07-distribution.md](07-distribution.md) で、CIをリリース向けに拡張し、`.dmg`配布までを行います。
