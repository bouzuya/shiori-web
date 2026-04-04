# AGENTS.md

## コーディングスタイル

- xxx/mod.rs ではなく xxx.rs + xxx/... 形式を取ること
- テストコードにおいて unwrap は使用せず、 ? operator と anyhow を使用する
- フィールドはソートすること
    - ただし Ord などで意味を持つ場合は除く。その場合はコメントで明示すること
- `#[derive(...)]` はソートすること
    - 例: `#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]`

## 開発スタイル

TDD (テスト駆動開発) で開発を進めること。

### TDD サイクル

1. **Red** — 失敗するテストを先に書く
2. **Green** — テストを通す最小限の実装を書く
3. **Refactor** — テストが通る状態を維持しつつコードを整理する

### 守るべきルール

- 実装コードを書く前に、必ず失敗するテストを書く
- テストが通る最小限のコードだけを書く。先回りして実装しない
- リファクタリングはテストが通っている状態でのみ行う
- 各ステップで `cargo test` を実行し、期待通りの結果 (Red では失敗、Green/Refactor では成功) を確認する

## テスト

- テストの実行: `cd /workspaces/shiori-web/backend && cargo test`
- テストは `#[cfg(test)] mod tests` 内にインラインで書く
