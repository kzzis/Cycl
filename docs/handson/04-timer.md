# Cycl ハンズオン 04: ポモドーロタイマー（Phase 3）

このフェーズでは、Todoとの連携（Phase 4）は一旦置いておき、**タイマー単体**を作ります。作業25分/短休憩5分/長休憩15分の切り替え、開始・一時停止・リセット、そしてRust側が状態を完全に保有する仕組みを実装します。

> **設計方針の再確認**
> 残り時間の減算は1秒ごとにRust側のバックグラウンドタスクが行い、その結果をTauriの **イベント** でフロントへ配信します。Dioxus側は受け取った値を表示するだけで、`setInterval`相当のポーリングは使いません。ウィンドウを閉じてもタイマーは正しく進み続けます。

## 1. 依存クレートを追加する

`src-tauri/Cargo.toml`:

```toml
[dependencies]
tokio = { version = "1", features = ["time"] }
```

## 2. タイマーの型と表示ヘルパーを `shared` クレートに置く

タイマーの状態遷移ロジックと`format_mm_ss`のような表示用ヘルパーは、フロント(残り時間の表示)とバックエンド(Phase 5でのトレイタイトル表示)の両方から使うため、最初から`shared`クレートに置きます。

`shared/src/timer.rs`(新規):

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimerPhase {
    Work,
    ShortBreak,
    LongBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerSettings {
    pub work_minutes: u32,
    pub short_break_minutes: u32,
    pub long_break_minutes: u32,
    pub sessions_before_long_break: u32,
}

impl Default for TimerSettings {
    fn default() -> Self {
        TimerSettings {
            work_minutes: 25,
            short_break_minutes: 5,
            long_break_minutes: 15,
            sessions_before_long_break: 4,
        }
    }
}

impl TimerSettings {
    pub fn duration_secs(&self, phase: TimerPhase) -> u32 {
        let minutes = match phase {
            TimerPhase::Work => self.work_minutes,
            TimerPhase::ShortBreak => self.short_break_minutes,
            TimerPhase::LongBreak => self.long_break_minutes,
        };
        minutes * 60
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerState {
    pub phase: TimerPhase,
    pub remaining_secs: u32,
    pub is_running: bool,
    pub completed_work_sessions: u32,
    pub settings: TimerSettings,
}

impl TimerState {
    pub fn new(settings: TimerSettings) -> Self {
        TimerState {
            remaining_secs: settings.duration_secs(TimerPhase::Work),
            phase: TimerPhase::Work,
            is_running: false,
            completed_work_sessions: 0,
            settings,
        }
    }

    /// 現フェーズを終え、次のフェーズへ進める。作業セッションを既定回数
    /// （デフォルト4回）終えるごとに長い休憩を挟み、休憩の後は必ず作業に戻る。
    pub fn advance_phase(&mut self) {
        self.phase = match self.phase {
            TimerPhase::Work => {
                self.completed_work_sessions += 1;
                if self.completed_work_sessions % self.settings.sessions_before_long_break == 0 {
                    TimerPhase::LongBreak
                } else {
                    TimerPhase::ShortBreak
                }
            }
            TimerPhase::ShortBreak | TimerPhase::LongBreak => TimerPhase::Work,
        };
        self.remaining_secs = self.settings.duration_secs(self.phase);
        self.is_running = false;
    }

    pub fn reset_current_phase(&mut self) {
        self.remaining_secs = self.settings.duration_secs(self.phase);
        self.is_running = false;
    }
}

/// 残り秒数を`mm:ss`形式にする。フロントのリング表示・Phase 5のトレイタイトル表示の両方で使う。
pub fn format_mm_ss(total_seconds: u32) -> String {
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// フェーズの日本語ラベル(フロント表示専用)。
pub fn phase_label(phase: TimerPhase) -> &'static str {
    match phase {
        TimerPhase::Work => "作業",
        TimerPhase::ShortBreak => "短い休憩",
        TimerPhase::LongBreak => "長い休憩",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> TimerSettings {
        TimerSettings::default()
    }

    #[test]
    fn work_session_moves_to_short_break_by_default() {
        let mut state = TimerState::new(settings());
        state.advance_phase();
        assert_eq!(state.phase, TimerPhase::ShortBreak);
        assert_eq!(state.completed_work_sessions, 1);
    }

    #[test]
    fn fourth_work_session_moves_to_long_break() {
        let mut state = TimerState::new(settings());
        for _ in 0..3 {
            state.advance_phase(); // Work -> ShortBreak
            state.advance_phase(); // ShortBreak -> Work
        }
        state.advance_phase(); // 4回目のWork -> LongBreak
        assert_eq!(state.phase, TimerPhase::LongBreak);
        assert_eq!(state.completed_work_sessions, 4);
    }

    #[test]
    fn break_always_returns_to_work() {
        let mut state = TimerState::new(settings());
        state.advance_phase(); // -> ShortBreak
        state.advance_phase(); // -> Work
        assert_eq!(state.phase, TimerPhase::Work);
    }

    #[test]
    fn reset_current_phase_restores_full_duration_and_stops() {
        let mut state = TimerState::new(settings());
        state.remaining_secs = 10;
        state.is_running = true;
        state.reset_current_phase();
        assert_eq!(state.remaining_secs, 25 * 60);
        assert!(!state.is_running);
    }

    #[test]
    fn format_mm_ss_pads_with_zero() {
        assert_eq!(format_mm_ss(65), "01:05");
        assert_eq!(format_mm_ss(600), "10:00");
    }
}
```

状態遷移という「ロジック」は非同期処理と切り離した純粋な関数にしているので、`tokio`ランタイムなしで高速にテストできます。テストは`shared`クレートの中にあるため、フロント(wasm)・バックエンド(ネイティブ)どちらのビルド設定にも影響されず`cargo test -p shared`で実行できます。

`shared/src/lib.rs` を更新します。

```rust
mod timer;
mod todo;

pub use timer::{format_mm_ss, phase_label, TimerPhase, TimerSettings, TimerState};
pub use todo::Todo;
```

## 3. タイマーエンジン（バックグラウンドタスク）

`src-tauri/src/timer/engine.rs`:

```rust
use shared::{TimerSettings, TimerState};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tokio::time::{interval, Duration};

pub struct TimerEngine {
    state: Arc<Mutex<TimerState>>,
}

impl TimerEngine {
    pub fn new(app_handle: AppHandle) -> Self {
        let state = Arc::new(Mutex::new(TimerState::new(TimerSettings::default())));
        spawn_tick_loop(app_handle, state.clone());
        TimerEngine { state }
    }

    pub fn snapshot(&self) -> TimerState {
        self.state.lock().unwrap().clone()
    }

    pub fn start(&self) -> TimerState {
        let mut state = self.state.lock().unwrap();
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
        state.clone()
    }
}

fn spawn_tick_loop(app_handle: AppHandle, state: Arc<Mutex<TimerState>>) {
    tauri::async_runtime::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;

            let snapshot = {
                let mut state = state.lock().unwrap();
                if state.is_running {
                    if state.remaining_secs > 0 {
                        state.remaining_secs -= 1;
                    }
                    if state.remaining_secs == 0 {
                        state.advance_phase();
                    }
                }
                state.clone()
            };

            let _ = app_handle.emit("timer:tick", &snapshot);
        }
    });
}
```

ポイントは2つです。

1. `tauri::async_runtime::spawn` を使うことで、Tauriが内部で持っている非同期ランタイム上にタスクを乗せます。自前でtokioランタイムを構築する必要はありません。
2. `Mutex` のロックは値を読み書きして `clone()` するところまでで終え、`emit` はロックを外れた後に呼びます。ロックを保持したまま非同期処理を挟まないのがポイントです。

`snapshot`/`start`/`pause`/`reset` はTauriコマンドから直接呼ぶ同期メソッドです。バックグラウンドの1秒tickループとは別に、いつでも即座に状態を読み書きできます。

`src-tauri/src/timer/mod.rs`:

```rust
pub mod engine;
```

(`state`モジュールはなくなりました。型は`shared`クレートから使います。)

## 4. Tauriコマンド

`src-tauri/src/commands/timer.rs`:

```rust
use shared::TimerState;
use crate::timer::engine::TimerEngine;
use tauri::State;

#[tauri::command]
pub fn timer_get_state(engine: State<TimerEngine>) -> TimerState {
    engine.snapshot()
}

#[tauri::command]
pub fn timer_start(engine: State<TimerEngine>) -> TimerState {
    engine.start()
}

#[tauri::command]
pub fn timer_pause(engine: State<TimerEngine>) -> TimerState {
    engine.pause()
}

#[tauri::command]
pub fn timer_reset(engine: State<TimerEngine>) -> TimerState {
    engine.reset()
}
```

`src-tauri/src/commands/mod.rs`:

```rust
pub mod timer;
pub mod todo;
```

## 5. `lib.rs` を更新する

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
        .setup(|app| {
            let app_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("cycl.sqlite3");
            let conn = db::open(&db_path)?;
            app.manage(AppState {
                db: Arc::new(Mutex::new(conn)),
            });
            app.manage(TimerEngine::new(app.handle().clone()));
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

## 6. フロントのIPCラッパーにタイマー系を追加する

`src/tauri_api.rs` に追記します(`invoke`/`invoke0`/`todo`モジュールはPhase 2のまま)。

```rust
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
```

## 7. `timer:tick` イベントを購読する仕組みを作る

> **概念: フロント(wasm)でのイベント購読**
> ReactでのTauriイベント購読は`@tauri-apps/api/event`の`listen()`をimportするだけでした。Dioxus(wasm)では、`window.__TAURI__.event.listen`を`wasm-bindgen`経由で呼び、JS側に渡すコールバックは`wasm_bindgen::closure::Closure`として作ります。Cyclのタイマー・Todo更新通知はアプリのライフタイム全体で1つだけ購読し続ければよいので、`Closure`は`forget()`して意図的にリークします(アプリごとに高々数個なので実質的な問題にはなりません)。

`src/tauri_api.rs` の先頭のuse文を拡張し、`listen`関数を追加します。

```rust
use serde::{de::DeserializeOwned, Serialize};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn invoke_raw(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn listen_raw(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> Result<JsValue, JsValue>;
}

// ...(invoke_inner/invoke0/invokeはPhase 2のまま)

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
```

`js-sys`はルートの`Cargo.toml`に既に依存として入っています(01ハンズオンのscaffold時点で追加済み)。

## 8. `use_timer` フック

`src/hooks/use_timer.rs`(新規):

```rust
use dioxus::prelude::*;
use shared::TimerState;

use crate::tauri_api::{self, timer as api};

#[derive(Clone, Copy)]
pub struct UseTimer {
    pub state: Signal<Option<TimerState>>,
}

impl UseTimer {
    pub fn start(&self) {
        spawn(async {
            let _ = api::start_timer().await;
        });
    }

    pub fn pause(&self) {
        spawn(async {
            let _ = api::pause_timer().await;
        });
    }

    pub fn reset(&self) {
        spawn(async {
            let _ = api::reset_timer().await;
        });
    }
}

pub fn use_timer() -> UseTimer {
    let hook = UseTimer {
        state: use_signal(|| None),
    };

    use_effect(move || {
        let mut initial_state = hook.state;
        spawn(async move {
            if let Ok(initial) = api::get_timer_state().await {
                initial_state.set(Some(initial));
            }
        });

        let mut ticked_state = hook.state;
        tauri_api::listen::<TimerState>("timer:tick", move |snapshot| {
            ticked_state.set(Some(snapshot));
        });
    });

    hook
}
```

マウント時に一度 `timer_get_state` を呼んで即座に現在値を描画し(次のtickまで最大1秒待たされないように)、以降は `timer:tick` イベントで更新し続けます。`start`/`pause`/`reset`は結果を待たずに送りっぱなしにします(1秒後の次のtickで結果が反映されます)。

`src/hooks/mod.rs`:

```rust
pub mod use_timer;
pub mod use_todos;
```

## 9. リング型プログレスUI

`src/components/pomodoro_timer.rs`(新規):

```rust
use dioxus::prelude::*;
use shared::{format_mm_ss, phase_label};

use crate::hooks::use_timer::use_timer;

const RADIUS: f64 = 90.0;

#[component]
pub fn PomodoroTimer() -> Element {
    let timer = use_timer();
    let Some(state) = timer.state.read().clone() else {
        return rsx! { p { class: "muted", "読み込み中..." } };
    };

    let circumference = 2.0 * std::f64::consts::PI * RADIUS;
    let total = state.settings.duration_secs(state.phase) as f64;
    let progress = if total == 0.0 {
        0.0
    } else {
        (total - state.remaining_secs as f64) / total
    };
    let offset = circumference * (1.0 - progress);

    rsx! {
        div { class: "pomodoro",
            p { class: "pomodoro__phase", "{phase_label(state.phase)}" }
            div { class: "pomodoro__ring",
                svg {
                    width: "220", height: "220", view_box: "0 0 220 220",
                    class: "pomodoro__svg",
                    circle {
                        cx: "110", cy: "110", r: "{RADIUS}",
                        fill: "none", stroke: "currentColor", stroke_width: "12",
                        class: "pomodoro__track",
                    }
                    circle {
                        cx: "110", cy: "110", r: "{RADIUS}",
                        fill: "none", stroke: "currentColor", stroke_width: "12",
                        stroke_linecap: "round",
                        stroke_dasharray: "{circumference}",
                        stroke_dashoffset: "{offset}",
                        class: "pomodoro__progress",
                    }
                }
                span { class: "pomodoro__remaining", "{format_mm_ss(state.remaining_secs)}" }
            }
            div { class: "pomodoro__controls",
                if state.is_running {
                    button { onclick: move |_| timer.pause(), "一時停止" }
                } else {
                    button { onclick: move |_| timer.start(), "開始" }
                }
                button { onclick: move |_| timer.reset(), "リセット" }
            }
        }
    }
}
```

Dioxusは素のSVG要素をそのまま`rsx!`で書けるので、React版とほぼ1:1の構造です。円全体を`pomodoro__svg`クラスで`-90deg`回転させ、残り時間の文字はリングの回転の影響を受けないよう`<svg>`の外側(`pomodoro__remaining`の`<span>`、CSSで`position: absolute`配置)に置きます。

`src/components/mod.rs`:

```rust
mod pomodoro_timer;
mod todo_form;
mod todo_item;
mod todo_list;

pub use pomodoro_timer::PomodoroTimer;
pub use todo_form::TodoForm;
pub use todo_item::TodoItem;
pub use todo_list::TodoList;
```

`src/app.rs`:

```rust
#![allow(non_snake_case)]

use dioxus::prelude::*;

use crate::components::{PomodoroTimer, TodoList};

static CSS: Asset = asset!("/assets/styles.css");

pub fn App() -> Element {
    rsx! {
        link { rel: "stylesheet", href: CSS }
        main { class: "app",
            PomodoroTimer {}
            TodoList {}
        }
    }
}
```

`assets/styles.css` に追記します。

```css
.pomodoro {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1.5rem;
  margin-bottom: 2rem;
}

.pomodoro__phase {
  color: var(--muted-foreground);
  font-size: 1.1rem;
}

.pomodoro__ring {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
}

.pomodoro__svg {
  transform: rotate(-90deg);
}

.pomodoro__track {
  color: var(--border);
}

.pomodoro__progress {
  color: var(--primary);
  transition: stroke-dashoffset 1s linear;
}

.pomodoro__remaining {
  position: absolute;
  font-size: 1.8rem;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.pomodoro__controls {
  display: flex;
  gap: 0.5rem;
}
```

## 10. 動作確認

```bash
cargo test --workspace
cargo tauri dev
```

「開始」を押すとリングが少しずつ進み、残り時間が減っていくことを確認してください。「一時停止」で止まる、「リセット」で現在のフェーズの最初に戻ることも確認します。25分は長いので、確認時だけ`shared::timer::TimerSettings::default()`の`work_minutes`を一時的に小さい値(例: `1`)に変えて試すと早く確認できます。確認が終わったら元に戻してください。

> **テストについての補足**: フェーズ遷移のロジック(`TimerState::advance_phase`など)は純粋関数として`shared`クレートの単体テストで検証済みです。一方、1秒ごとのtickループ自体は非同期タスクの時間経過に依存するため、自動テストではなく上記の手動確認でカバーします。すべてを自動テスト化するのではなく、ロジックはテストで、時間依存のI/O的な部分は手動確認でと使い分けています。

## 11. コミットする

```bash
git add .
git commit -m "feat: add rust-driven pomodoro timer engine with ring progress UI"
```

## OSSチェックポイント

- [ ] `cargo test --workspace` が通る(フェーズ遷移ロジックのテスト含む)
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` が通る
- [ ] 開始・一時停止・リセットを実アプリで確認した
- [ ] ウィンドウを閉じて再度開いてもタイマーが正しい残り時間を示すことを確認した

次は [05-integration.md](05-integration.md) で、このタイマーとTodoを連携させます。
