# Cycl ハンズオン 00: 全体像と環境準備

このハンズオンは [`docs/roadmap.md`](../roadmap.md) のフェーズ構成に沿って、macOSメニューバー常駐のポモドーロ×Todoアプリ「**Cycl**」を、写経しながら最初から最後まで作り切るための手順書です。

コードは全文掲載します。読みながらそのまま入力・コピーしていけば、各フェーズの終わりで実際に動くアプリが手元に残ります。あわせて、テスト・Lint・CI・ドキュメントなど「よく管理されたOSS」に必要な要素も、後付けではなく各フェーズの中に組み込みます。

## 対象読者

- プログラミング経験はある（言語は問わない。React/TypeScriptの経験があれば一部の概念がイメージしやすいですが必須ではありません）
- Rust・Tauri・Dioxusはこれが初めて

Rustの所有権・型システムや、Tauri固有の概念（コマンド、`State`、`Emitter`、IPC、capabilities）、Dioxus/wasm-bindgen固有の概念（`rsx!`、シグナル、wasmとJSの相互呼び出し）が出てくるたびに、都度簡単な補足を挟みます。既にRust/Tauri/Dioxusに詳しい場合は読み飛ばして構いません。

## 完成イメージ

[`requirements.md`](../../requirements.md) にある機能要件をすべて満たします。

- Todoの作成・編集・削除・完了切替
- 「現在取り組むTodo」の選択
- ポモドーロタイマー（作業25分 / 短休憩5分 / 長休憩15分、設定変更可）、開始・一時停止・リセット
- リング型プログレスUI、ダークテーマ
- Todo単位でのポモドーロ実行回数記録・目標回数表示（🍅×3）
- セッション終了時のmacOS通知
- メニューバー常駐、残り時間表示、ウィンドウ表示/非表示切替
- GitHub Actionsによる `.dmg` ビルド・Releases配布

## 技術スタックと確定事項

| レイヤー | 技術 |
|---|---|
| デスクトップフレームワーク | Tauri v2 |
| バックエンド（ロジック） | Rust |
| フロントエンド（UI） | Dioxus（Rust → WebAssembly） |
| UIコンポーネント / スタイリング | 素のCSS（手書き、フレームワークなし） |
| 型・ロジックの共有 | `shared` クレート（フロント・バックエンド間でTodo/タイマーの型と表示用ヘルパーを共有） |
| データ永続化 | SQLite（**rusqlite** を直接使用） |
| パッケージ管理 | cargo に統一（フロント・バックエンドとも）。ビルドツールは **Dioxus CLI (`dx`)** |
| 配布 | GitHub Releases（`.dmg`） |

ブレインストーミングで決めたこのハンズオン固有の方針です。

1. **ロジックはすべてRust側に閉じる。** ロードマップには `tauri-plugin-sql` の導入とありますが、このプラグインは本来フロントJSから `db.execute()` でSQLを直接発行する用途のものです。今回は採用せず、**`rusqlite` をRustコマンドの中だけで使い**、フロントは型付きの `invoke()` 呼び出しのみを行います。SQL文はフロントに一切現れません。フロントもRust（Dioxus/wasm）になりますが、`src-tauri`（ネイティブ）と`src`（wasm）は別バイナリとしてコンパイルされる別プロセス相当なので、直接関数を呼び合うことはできず、引き続きTauriのIPC（`invoke`/イベント）を介した通信になります。
2. **タイマーの時間進行もRust側が真実の情報源（single source of truth）を持つ。** `tokio` のバックグラウンドタスクが1秒ごとにtickし、残り時間・フェーズ・実行状態を Tauri のイベントでフロントへ配信します。Dioxus側はその値を表示するだけの薄いView層です。開始・一時停止・リセットはすべて `invoke()` でRustに指示します。ウィンドウを閉じてもタイマーは正しく進み続けます。
3. **通知の発火判定もRust側。** セッション終了はRustが検知し、`tauri-plugin-notification` をRustから呼び出します。フロントに終了判定ロジックは持たせません。
4. **型と表示用ヘルパーは `shared` クレートで一元管理する。** フロント・バックエンドが両方Rustになったことを活かし、`Todo` や `TimerState` などのstruct、`format_mm_ss` のような純粋な表示ヘルパー関数は `shared` クレートに置き、両クレートから同じ定義を`use`します。（Reactだった頃は`src/lib/types.ts`にRustの構造体を手書きで複製する必要がありましたが、その二重管理が丸ごと不要になります。）

## ハンズオンの構成

| ファイル | フェーズ | 内容 |
|---|---|---|
| [01-setup.md](01-setup.md) | Phase 0 | プロジェクト雛形、`shared`クレート、Lint/Format、CI雛形 |
| [02-data-layer.md](02-data-layer.md) | Phase 1 | rusqliteスキーマ・マイグレーション、Todo/SessionのTauriコマンド、Rustテスト |
| [03-todo-ui.md](03-todo-ui.md) | Phase 2 | Todoリスト UI、フロントのコンポーネントテスト |
| [04-timer.md](04-timer.md) | Phase 3 | ポモドーロタイマーエンジン（Rust）、リング型プログレスUI |
| [05-integration.md](05-integration.md) | Phase 4 | Todo×タイマー連携、セッション記録、macOS通知 |
| [06-tray.md](06-tray.md) | Phase 5 | メニューバー常駐 |
| [07-distribution.md](07-distribution.md) | Phase 6 | CI本格化、CHANGELOG運用、`.dmg`配布 |

各フェーズの最後に **「OSSチェックポイント」** として、テスト・Lintが通るか、READMEやCHANGELOGの更新が必要かを確認する項目を置いています。

## 開発環境の準備

以下がインストール済みであることを確認してください。

```bash
rustc -V     # 1.77+
cargo -V
rustup target list --installed | grep wasm32-unknown-unknown   # フロント(Dioxus)をwasmにコンパイルするために必要
dx --version           # Dioxus CLI（フロントのビルド・開発サーバー）
cargo tauri --version  # Tauri CLI（`cargo tauri dev`/`build`用）
xcode-select -p   # Xcode Command Line Toolsが入っていればパスが表示される
```

入っていないものがあれば用意します。

```bash
# Rust（未インストールの場合）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# フロントをwasmにコンパイルするためのターゲット
rustup target add wasm32-unknown-unknown

# Xcode Command Line Tools（未インストールの場合）
xcode-select --install
```

`dx`(Dioxus CLI)と`cargo tauri`(Tauri CLI)は素の`cargo install`ではなく、**miseの`cargo:`バックエンド**でプロジェクトスコープに入れます(node/go/python/rustと同じ感覚でバージョン管理でき、`~/.cargo/bin`をグローバルに汚しません)。

```bash
cd /Users/k/Dev/Cycl
mise use cargo:dioxus-cli@0.6 cargo:tauri-cli@2
```

これでリポジトリ直下に`mise.toml`が作られ、このディレクトリ配下にいる時だけ`dx`/`cargo tauri`がPATHに乗ります。

> `wasm32-unknown-unknown`が何をするターゲットかは[01-setup.md](01-setup.md)の中で改めて補足します。フロント・バックエンドとも最終的にRust/cargoだけで完結しますが、プロジェクトの雛形を生成する最初の一回だけ`pnpm create tauri-app`（Node製のスキャフォールドツール）を使います。生成されるプロジェクト自体はNode非依存です。

Tauri v2はmacOSで追加のシステム依存はありませんが、上記が揃っていることが前提です。

## リポジトリの雛形を整える

「よく管理されたOSS」を目指すため、実装に入る前にリポジトリの土台を作ります。

```bash
cd /Users/k/Dev/Cycl
git init
```

### `.gitignore`

Tauri/Rust/Dioxusの生成物を除外します。

```gitignore
# Rust / Tauri / Dioxus（ワークスペース共通のビルド出力）
/target/
/dist/
/gen/schemas

# エディタ・OS
.DS_Store
.vscode/
!.vscode/extensions.json

# ローカル設定
docs/
```

> `Cargo.lock` はアプリケーション（ライブラリではない）なので、再現可能なビルドのためにコミット対象にします（ignoreしません）。
>
> 既存の `docs/` を除外している行は、内部ドキュメント（ロードマップやこのハンズオン自体）をリポジトリ公開時に含めたくない場合の設定です。OSS公開時にハンズオンやロードマップも公開したい場合はこの行を削除してください。

### `README.md`

```markdown
# Cycl

macOSメニューバー常駐のポモドーロ×Todoアプリ。TodoとポモドーロセッションをRustで管理し、Dioxus（Rust→WebAssembly）でUIを描画します。

## 機能

- Todoの作成・編集・削除・完了管理
- Todo単位でのポモドーロ実行回数記録
- ポモドーロタイマー（作業/短休憩/長休憩、時間は設定変更可）
- メニューバー常駐・残り時間表示
- セッション終了時のmacOS通知

## 技術スタック

Tauri v2 / Rust / Dioxus (Rust → WebAssembly) / SQLite (rusqlite)

## 開発

```bash
cargo tauri dev
```

## テスト

```bash
cargo test --workspace
```

## ビルド

```bash
cargo tauri build
```

## ライセンス

[MIT](./LICENSE)
```

### `LICENSE`（MIT）

```text
MIT License

Copyright (c) 2026 <あなたの名前>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

`<あなたの名前>` は実際の名前・GitHubユーザー名に置き換えてください。

### `CONTRIBUTING.md`

```markdown
# Contributing

## セットアップ

`docs/handson/01-setup.md` を参照してください。

## コミット規約

[Conventional Commits](https://www.conventionalcommits.org/) に従います。

- `feat: ` 新機能
- `fix: ` バグ修正
- `refactor: ` 挙動を変えないコード変更
- `test: ` テストの追加・修正
- `docs: ` ドキュメントのみの変更
- `chore: ` ビルド・CI・依存関係など

例: `feat: add todo completion toggle`

## プルリクエストを送る前に

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

すべて通ることを確認してください。CIでも同じチェックが走ります。
```

### `CHANGELOG.md`

[Keep a Changelog](https://keepachangelog.com/) 形式で始めます。

```markdown
# Changelog

すべての注目すべき変更をこのファイルに記録します。
フォーマットは [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に、
バージョニングは [Semantic Versioning](https://semver.org/lang/ja/) に従います。

## [Unreleased]

### Added

- プロジェクト雛形
```

### 最初のコミット

```bash
git add .gitignore README.md LICENSE CONTRIBUTING.md CHANGELOG.md
git commit -m "chore: initial repository scaffold"
```

ここまでできたら [01-setup.md](01-setup.md) に進み、実際にTauriプロジェクトを作成します。
