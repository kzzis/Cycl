# Cycl

macOSメニューバー常駐のポモドーロ×Todoアプリ。TodoとポモドーロセッションをRustで管理し、React/TypeScriptでUIを描画します。

## 機能

- Todoの作成・編集・削除・完了管理
- Todo単位でのポモドーロ実行回数記録
- ポモドーロタイマー（作業/短休憩/長休憩、時間は設定変更可）
- メニューバー常駐・残り時間表示
- セッション終了時のmacOS通知

## 技術スタック

Tauri v2 / Rust / React + TypeScript / Tailwind CSS + shadcn/ui / SQLite (rusqlite)

## 開発

```bash
pnpm install
pnpm tauri dev
```

## テスト

```bash
pnpm test          # フロントエンド (vitest)
cd src-tauri && cargo test   # Rust
```

## ビルド

```bash
pnpm tauri build
```

## ライセンス

[MIT](./LICENSE)
```

### `LICENSE`（MIT）

```text
MIT License

Copyright (c) 2026 kzzis

