# Cycl ロードマップ

元要件: `/Users/k/Downloads/requirements.md`

## Phase 0: プロジェクト雛形

- Tauri v2 + React + TypeScript のプロジェクト作成
- Tailwind + shadcn/ui のセットアップ
- tauri-plugin-sql 導入、SQLite接続確認
- 最低限のウィンドウが起動することを確認

## Phase 1: データ層（Rust側）

- `Todo` / `PomodoroSession` のSQLiteスキーマ作成・マイグレーション
- Todo CRUD用のTauriコマンド実装（作成・編集・削除・完了切替）
- ポモドーロセッション記録用のTauriコマンド実装

## Phase 2: Todoリスト UI

- shadcn/uiベースのTodo一覧・追加・編集・削除UI
- 完了状態の切替UI
- 「現在取り組むTodo」の選択UI
- Todo一覧に実行済みポモドーロ数表示（🍅×3）、目標数表示（任意設定）

## Phase 3: ポモドーロタイマー

- タイマーロジック（作業25分/短休憩5分/長休憩15分、設定変更可能）
- 開始・一時停止・リセット操作
- リング型プログレスUI（SVG/Canvas）
- ダークテーマ対応

## Phase 4: Todo × タイマー連携

- 選択中Todoに対してタイマー実行 → セッション記録
- セッション終了時にTodoのpomodoro_countを更新
- セッション終了時のmacOS通知

## Phase 5: メニューバー常駐

- トレイアイコン常駐
- タイマー実行中は残り時間をメニューバーに表示
- メニューからウィンドウ表示・非表示切替

## Phase 6: 配布

- GitHub Actions ワークフロー（tauri build → .dmg生成）
- mainブランチpushでビルド→Releasesへアップロード
- 公証なし配布の初回起動手順をREADMEに記載
- ポートフォリオサイトからのリンク設置（別リポジトリ側の作業）

## 進め方

各フェーズごとに、着手前に簡単な設計確認 → 実装計画を作成してから実装する。
