# Cycl ハンズオン 07: 配布（Phase 6）

最後に、GitHubへの公開、タグをきっかけにした `.dmg` ビルド・GitHub Releasesへの自動アップロード、そして公証なし配布の初回起動手順までを整えます。

## 1. GitHubにリポジトリを作成してpushする

`gh` コマンドが使える場合:

```bash
gh repo create cycl --public --source=. --remote=origin --push
```

`gh` がない場合は GitHub上で空リポジトリを作成し、以下でpushします。

```bash
git remote add origin git@github.com:<あなたのユーザー名>/cycl.git
git branch -M main
git push -u origin main
```

> **一度だけ確認すること**: GitHubリポジトリの Settings → Actions → General → *Workflow permissions* が "Read and write permissions" になっているか確認してください。後述のリリースワークフローがGitHub Releasesへアップロードするために書き込み権限が必要です。

## 2. これまでのCIの最終形を確認する

Phase 0〜3で少しずつ追加してきた `.github/workflows/ci.yml` は、この時点で以下の形になっています(差分ではなく全体を掲載します。中身が一致しているか確認してください)。React版では「フロント(pnpm)」と「バックエンド(cargo)」でジョブが分かれていましたが、フロントもRustになったため**ワークスペース全体を対象にした単一のcargoジョブ**に統合されています。

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
      - run: cargo test --workspace

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

`build` が `rust-lint` に依存しているため、Lintとテストが通らない限り、mainブランチへのマージ後もビルド確認まで進みません。`dioxus-cli`/`tauri-cli`のインストールはビルドのたびに数分かかるため、余裕があれば`actions/cache`で`~/.cargo/bin`をキャッシュする最適化を検討してもよいですが、最初はシンプルな形で始めます。

## 3. リリース用ワークフローを作る

タグ `v*.*.*` がpushされたときだけ動く、リリース専用のワークフローを追加します。公式の `tauri-apps/tauri-action` が、ビルド・`.dmg`生成・GitHub Releasesへのアップロードまでを行ってくれます。

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - "v*.*.*"

jobs:
  release:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - run: cargo install dioxus-cli --locked
      - uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: ${{ github.ref_name }}
          releaseName: "Cycl ${{ github.ref_name }}"
          releaseBody: "詳細は CHANGELOG.md を参照してください。"
          releaseDraft: true
          prerelease: false
```

`tauri-apps/tauri-action` は内部で`tauri.conf.json`の`beforeBuildCommand`(`dx bundle --release`)を実行するため、`dioxus-cli`だけ事前にインストールしておけば十分です(アクション自体がRustツールチェーンのセットアップと`cargo tauri build`相当の処理を行うため、`tauri-cli`を別途インストールする必要はありません)。`wasm32-unknown-unknown`ターゲットの追加は必須です。

`releaseDraft: true` にしているのは、公開前にリリースノートを見直せるようにするためです。ドラフトの内容を確認してから手動でPublishします。

今回は公証(Apple Developer Program によるnotarization)を行わないため、署名関連の設定(`APPLE_CERTIFICATE`等のsecrets)は不要です。

## 4. バージョニングとCHANGELOGの運用

Cyclでは [Conventional Commits](https://www.conventionalcommits.org/) でコミットしているため、`feat:` は minor、`fix:` は patch、破壊的変更を含む場合は `!` を付けて major を上げる、という [Semantic Versioning](https://semver.org/lang/ja/) の対応がそのまま使えます。

リリースするときの手順:

1. `CHANGELOG.md` の `[Unreleased]` にある項目を、バージョン番号と日付を持つ新しいセクションに移す

```markdown
## [Unreleased]

## [0.1.0] - 2026-07-05

### Added

- プロジェクト雛形
- Todoの作成・編集・削除・完了管理
- ポモドーロタイマー(Rust実装)とTodo連携、macOS通知
- メニューバー常駐
- GitHub ActionsによるCI/CD

[Unreleased]: https://github.com/<あなたのユーザー名>/cycl/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/<あなたのユーザー名>/cycl/releases/tag/v0.1.0
```

2. バージョン番号を揃える。全クレートがCargoワークスペースになっているので、ここで**バージョン管理を1箇所に集約**します。

   ルート `Cargo.toml` に `[workspace.package]` を追加し、`version`をそこに持たせます。

   ```toml
   [workspace]
   members = ["src-tauri", "shared"]

   [workspace.package]
   version = "0.1.0"
   ```

   ルート・`shared`・`src-tauri` それぞれの `[package]` セクションの `version = "0.1.0"` を `version.workspace = true` に変更します(初回だけの一手間です。以降のリリースではルートの`[workspace.package] version`を書き換えるだけで全クレートに反映されます)。

   ```toml
   [package]
   name = "cycl"        # クレートごとに異なる。versionだけワークスペースから継承する
   version.workspace = true
   ```

   `src-tauri/tauri.conf.json` の `version` だけは別途揃える必要があります。

   ```json
   {
     "version": "0.1.0"
   }
   ```

3. リリースコミットを作り、タグを打ってpushする

```bash
git add CHANGELOG.md Cargo.toml src-tauri/Cargo.toml shared/Cargo.toml src-tauri/tauri.conf.json
git commit -m "chore(release): v0.1.0"
git tag v0.1.0
git push origin main --tags
```

タグのpushをトリガーに `release.yml` が動き、GitHub ReleasesにドラフトとDMGが作成されます。内容を確認してPublishしてください。

> 将来的にリリースノート生成やバージョン計算まで自動化したい場合は [release-please](https://github.com/googleapis/release-please) のようなツールを追加で検討できますが、個人開発の規模ではこの手動フローで十分です。

## 5. 公証なし配布の初回起動手順をREADMEに書く

Apple Developer Programに加入していないため、配布する `.dmg` は署名されていても公証(notarization)はされません。そのままではmacOSのGatekeeperがブロックするので、`README.md` に案内を追記します。

```markdown
## インストール

[Releases](https://github.com/<あなたのユーザー名>/cycl/releases) から最新の `.dmg` をダウンロードし、Cycl.appを `/Applications` にコピーしてください。

このアプリは公証(Apple Notarization)を受けていないため、初回起動時にmacOSが起動をブロックします。以下のいずれかの方法で許可してください。

**方法A: ターミナルで隔離属性を外す(推奨)**

```bash
xattr -cr /Applications/Cycl.app
```

**方法B: システム設定から許可する**

1. Cycl.appをダブルクリックし、警告が出ることを確認する
2. システム設定 → プライバシーとセキュリティ を開く
3. 「"Cycl"は開発元を確認できないため使用がブロックされました」の右にある「このまま開く」をクリックする
```

## 6. コミットする

```bash
git add .
git commit -m "docs: add release workflow and first-launch instructions"
```

## OSSチェックポイント

- [ ] `main` へのpushで `ci.yml` が緑になる
- [ ] タグ `v0.1.0` のpushで `release.yml` が動き、ドラフトリリースに `.dmg` が添付される
- [ ] `CHANGELOG.md` にバージョンごとの変更点が記録されている
- [ ] ルート`Cargo.toml`の`[workspace.package] version` と `tauri.conf.json` のバージョンが一致している
- [ ] READMEの初回起動手順どおりに、実際にダウンロードした `.dmg` から起動できることを確認した

---

これで [roadmap.md](../roadmap.md) の Phase 0〜6 すべてが完成です。お疲れさまでした。
