# shiori-web

## コーディングスタイル

- モジュールは `xxx/mod.rs` ではなく `xxx.rs` + `xxx/` 形式を使うこと
- `unwrap` を使用しないこと
  - テストコードでは `?` 演算子と `anyhow` を使い、 `expect` や `unwrap` を使用しないこと
- フィールドはアルファベット順にソートすること
  - ただし `Ord` の実装など順序に意味がある場合は除く。その場合はコメントで明示すること
- `#[derive(...)]` の項目はアルファベット順にソートすること
  - 例: `#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]`
- テストは `#[cfg(test)] mod tests` 内にインラインで書くこと
- コード編集後は `cargo +nightly fmt` を実行すること

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

## コマンド

- format: `cargo +nightly fmt`
- lint: `cargo clippy -- -D warnings`
- test: `cargo test`
