# Cycl ハンズオン 01: プロジェクト雛形（Phase 0）

[00-overview.md](00-overview.md) でリポジトリの土台ができている前提で進めます。
ここではTauri + Dioxusのプロジェクトを作成し、`shared`クレート、Lint/Format、CIの雛形まで整えて
「最低限のウィンドウが起動する」状態を作ります。

## 1. Tauriプロジェクトを作成する

`pnpm create tauri-app` は指定した名前でサブディレクトリを新規作成します。
既に `/Users/k/Dev/Cycl` というリポジトリが存在するので、
一度隣に作ってから中身を移動します。

フロントエンドをRust(Dioxus)にすると、パッケージマネージャの選択肢も含めて生成物がNodeに一切依存しなくなるため、`--manager cargo` を明示します。

```bash
cd /Users/k/Dev
pnpm create tauri-app cycl-scaffold --manager cargo --template dioxus --identifier com.cycl.app
```

対話プロンプトで聞かれる場合は以下のように答えます（上記のフラグで全て指定済みなら聞かれません）。

```
✔ Choose which language to use for your frontend · Rust
✔ Choose your package manager · cargo
✔ Choose your UI template · Dioxus
```

> **生成される構成**: フロント(Dioxus)は**リポジトリのルート**に生成されます（`src/`配下、ルートの`Cargo.toml`自体がフロントのクレート）。バックエンド(Tauri)は従来どおり`src-tauri/`です。ルートの`Cargo.toml`が`[workspace] members = ["src-tauri"]`を持つため、この時点で既にCargoワークスペースになっています。

中身をリポジトリ直下に移動します。

```bash
cd /Users/k/Dev
rsync -a --exclude '.git' cycl-scaffold/ Cycl/
rm -rf cycl-scaffold
cd Cycl
```

### プロジェクト名を統一する

移動しただけだと `cycl-scaffold` という名前がいくつかの設定ファイルに残っているので、`cycl` に揃えます。

ルート `Cargo.toml`（フロントのクレート。`create-tauri-app`は自動で`-ui`サフィックスを付けます）:

```toml
[package]
name = "cycl-ui"
version = "0.1.0"
edition = "2021"

[dependencies]
dioxus = { version = "0.6", features = ["web"] }
dioxus-logger = "0.6"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = "0.3"
js-sys = "0.3"
serde = { version = "1", features = ["derive"] }
serde-wasm-bindgen = "0.6"

[workspace]
members = ["src-tauri"]
```

`src-tauri/Cargo.toml` の `[package]` セクション:

```toml
[package]
name = "cycl"
version = "0.1.0"
description = "A menu-bar pomodoro & todo app"
authors = ["you"]
edition = "2021"

[lib]
name = "cycl_lib"
crate-type = ["staticlib", "cdylib", "rlib"]
```

生成直後の`src-tauri/src/lib.rs`にある`pub fn run()`のシグネチャ等、クレート名変更に伴う参照箇所があれば`cycl_lib`に合わせます。

`src-tauri/tauri.conf.json`:

```json
{
  "productName": "Cycl",
  "identifier": "com.cycl.app",
  ...
}
```

`Dioxus.toml`（フロントのクレート名を変えたら、対応する`name`も揃えます）:

```toml
[application]
name = "cycl-ui"
default_platform = "web"
out_dir = "dist"
asset_dir = "assets"

[web.app]
title = "Cycl"

[web.watcher]
reload_html = true
watch_path = ["src", "assets"]
```

## 2. `shared` クレートを作る

フロント(`cycl-ui`)とバックエンド(`cycl_lib`)は別々にコンパイルされる別バイナリですが、**どちらも純粋なRustのstruct・関数なら共有できます**。Todoやタイマーの型、`format_mm_ss`のような表示用ヘルパーをここに置き、フロント側での型の手書き複製をなくします。

```bash
mkdir -p shared/src
```

`shared/Cargo.toml`:

```toml
[package]
name = "shared"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
```

> wasm32(フロント)とネイティブ(バックエンド)の両方でコンパイルされるクレートなので、プラットフォーム固有のクレート（`rusqlite`や`tokio`など）はここに置かないでください。serdeでシリアライズ可能な素のデータ型と、純粋関数だけを置きます。

`shared/src/lib.rs`（この時点では空でよい）:

```rust
// Todo/TimerStateなどの共有struct・表示用ヘルパーはPhase 2以降で追加します。
```

ルート `Cargo.toml` の `[workspace]` に `shared` を追加し、フロントからも参照できるようにします。

```toml
[dependencies]
# ...(既存のdioxus等はそのまま)
shared = { path = "shared" }

[workspace]
members = ["src-tauri", "shared"]
```

`src-tauri/Cargo.toml` の `[dependencies]` にも追加します。

```toml
[dependencies]
shared = { path = "../shared" }
```

## 3. 動作確認（最低限のウィンドウ起動）

```bash
cargo tauri dev
```

`tauri.conf.json` の `beforeDevCommand`(`dx serve --port 1420`)が自動的に実行され、Dioxus CLIがフロントをビルド・ホットリロードしながらTauriウィンドウが起動します。初回はRustのビルドに数分かかります。Tauriのデフォルトテンプレート画面（"Welcome to Tauri + Dioxus" とgreetボタン）が表示されれば成功です。確認できたら `Ctrl+C` で終了して構いません。

## 4. 手書きCSSとダークテーマ

Tailwind/shadcn/uiは使わず、素のCSSだけでスタイリングします。生成直後の`assets/styles.css`を、Cycl用のダークテーマベースに置き換えます。

```css
:root {
  font-family: -apple-system, "Helvetica Neue", Arial, sans-serif;
  font-size: 14px;
  line-height: 1.5;

  color-scheme: dark;
  --background: #1a1a1a;
  --foreground: #f5f5f5;
  --muted-foreground: #a3a3a3;
  --border: #333333;
  --primary: #6366f1;
  --primary-foreground: #ffffff;

  color: var(--foreground);
  background-color: var(--background);
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
}
```

Cyclはメニューバー常駐アプリとして常にダークテーマ前提なので、Reactハンズオン版のような`.dark`クラス切り替えは行わず、`:root`に直接ダーク配色を定義しています。

Dioxus側では、コンポーネントの中で`asset!()`マクロを使ってこのCSSを読み込みます（Vite/Trunkのような`index.html`は存在せず、Dioxus CLIがHTMLシェルを自動生成するため、CSSの読み込みもRustコードの中で行います）。

```rust
static CSS: Asset = asset!("/assets/styles.css");
```

```rust
rsx! {
    link { rel: "stylesheet", href: CSS }
    // ...
}
```

これは生成直後の`src/app.rs`に既にある書き方なので、Phase 2以降で自前のコンポーネントに置き換える際もこのパターンを踏襲します。

## 5. rustfmt / clippy を設定する

ワークスペース共通の設定として、リポジトリルートに置きます。

`rustfmt.toml`:

```toml
edition = "2021"
```

`src-tauri/src/lib.rs` の先頭にclippyの方針をコメントとして明示します（Cycl全体の既定として、警告をエラー扱いにする設定はCI側で行います）。

```rust
#![warn(clippy::all)]
```

フロントの`src/main.rs`にも同様に追加します。

```rust
#![warn(clippy::all)]
```

動作確認は**ワークスペース全体を1回のコマンドでチェックできます**（React版では`src-tauri`とフロントを別々にチェックしていましたが、両方Rustになったので不要になりました）。

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## 6. GitHub Actions CI の雛形を作る

この時点ではまだテストがないため、CIは **Lint + ビルド確認** のみです。テストジョブはPhase 1・2でそれぞれ追加します。

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

jobs:
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

  build:
    needs: [rust-lint]
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - run: cargo install dioxus-cli --locked
      - run: cargo install tauri-cli --version '^2.0.0' --locked
      - run: cargo tauri build --debug
```

`targets: wasm32-unknown-unknown` はフロントのwasmビルドに必須です。`dioxus-cli`/`tauri-cli`のインストールはビルドに数分かかるため、後でキャッシュ（`actions/cache`でcargoのbinディレクトリを保存する等）を検討してもよいですが、最初はシンプルに毎回インストールする形で始めます。

## 7. コミットする

```bash
git add .
git commit -m "feat: scaffold tauri v2 + dioxus project with shared crate"
```

## OSSチェックポイント

- [ ] `cargo fmt --all -- --check` が通る
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` が通る
- [ ] `cargo tauri dev` でウィンドウが起動する
- [ ] CIワークフローをpushし、GitHub Actionsが緑になることを確認した
- [ ] コミットメッセージがConventional Commitsに沿っている

次は [02-data-layer.md](02-data-layer.md) で、Rust側のデータ層を実装します。
