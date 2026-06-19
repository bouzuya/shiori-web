# shiori-web

## コーディングスタイル

- モジュールは `xxx/mod.rs` ではなく `xxx.rs` + `xxx/` 形式を使うこと
- `unwrap` を使用しないこと
  - テストコードでは `?` 演算子と `anyhow` を使い、 `expect` や `unwrap` を使用しないこと
- フィールドはアルファベット順にソートすること
  - ただし `Ord` の実装など順序に意味がある場合は除く。その場合はコメントで明示すること
- `#[derive(...)]` の項目はアルファベット順にソートすること
  - 例: `#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ::serde::Deserialize, ::serde::Serialize)]`
- HTML テンプレートの要素の属性はアルファベット順にソートすること
  - 例: `<html data-color-scheme="light" lang="ja">`
- テストは `#[cfg(test)] mod tests` 内にインラインで書くこと
- コード編集後は `cargo +nightly fmt` を実行すること
- 型にテスト用のインスタンス生成メソッド `for_test` を用意すること
  - `#[cfg(test)] impl T { pub fn for_test() -> Self { ... } }` の形で、全フィールドをランダム生成する
  - テストでは `T::for_test()` を基点とし、検証したいフィールドだけを構造体更新構文 (`T { field: ..., ..T::for_test() }`) で上書きして使う
  - テスト固有の値を毎回手書きせず、本質的な差分だけがテストに現れるようにするのが目的
- import / パス参照は「ワークスペース内」と「ワークスペース外」で扱いを分けること
  - **ワークスペース内 (`crate` 自身・`super`・`self`、およびワークスペースメンバ crate `kernel`) は `use` で取り込み、裸の名前 (`Xxx`) で参照する**
    - フルパスでの逐次参照や `::kernel::` のような絶対パス修飾はしない (`kernel` クレート自身の中では自己を crate 名で参照できないため、ワークスペース内 crate は一律 `use` に統一する)
    - クレートルートの再エクスポート (`crate::Xxx`) を使う。`crate::entities::` / `crate::read_models::` / `crate::use_cases::` などの内部モジュールパスを参照に直接書かない
    - ただしその型を定義しているモジュール内では、自分の型は `use` せず `Self` / 裸の型名で参照する (自己 import はローカル定義と衝突するため)
    - `use` は使用箇所に最も近いスコープへ置き、本体ビルドでの未使用 import 警告を避けること
      - 本体コードで使う型: ファイル先頭の `use`
      - テストでのみ使う型: `#[cfg(test)] mod tests` 内 (`use super::*;` の後) に `use`。`mod tests` の外にある `#[cfg(test)] fn for_test` でのみ使う型は、その関数本体に `use` を書く
  - **ワークスペース外 (外部依存 crate および `std`) は `use` せず、使用箇所で `::` 始まりの絶対パスでフルパス修飾して参照する**
    - 例: `::std::sync::Arc`、`::axum::http::StatusCode`、`#[derive(::serde::Deserialize)]`、`#[::tokio::test]`、`::tracing::error!(...)`
    - 先頭は必ず `::` を付けて crate ルートから辿る (ローカル定義との曖昧さを避けるため)。`axum::...` のような `::` なしの修飾にしない
    - メソッド呼び出しや derive のために trait をスコープへ入れる必要がある場合も `use` せず修飾する。メソッドは UFCS で呼ぶ
      - 例: `::askama::Template::render(&t)` / `::axum::response::IntoResponse::into_response(x)` / `<CookieJar as ::axum::extract::FromRequestParts<S>>::from_request_parts(..)` / `::rand::RngExt::random_range(&mut rng, ..)`
    - 唯一の例外: `use ::anyhow::Context as _;` のみ `use` を許可する (`?` 直前の `.context()` 利用が頻出で、UFCS 化すると著しく冗長になるため)
- `use` / 再エクスポートでグロブ (`::*`) を使わず、項目を1つずつ列挙すること
  - 例: `pub use self::entities::Bookmark;` のように個別に書く。`pub use self::entities::*;` は禁止
  - 唯一の例外: `#[cfg(test)] mod tests` 冒頭の `use super::*;` のみ許可する (テストから親モジュールを取り込む慣用)
  - 通常の `use foo::*;` は clippy lint `wildcard_imports` (`[workspace.lints.clippy]` で `deny`) が検出する (`use super::*;` は既定で除外)
    - ただしグロブ**再エクスポート** (`pub use foo::*;`) はこの lint の対象外。再エクスポートはこのルールに従い人手・レビューで防ぐこと

## CSS の構成方針

> **適用範囲は CSS のみ**。ここで述べる「locality 優先・重複許容」はスタイルシート固有の割り切りであり、Rust コードの設計判断 (凝集・モジュール分割・依存関係) には一切持ち込まないこと。Rust 側の設計はこの方針と無関係に判断する。

`crates/main/assets/index.css` は「ページ単位で完結させる」方針で構成すること。

- 各ページのテンプレートは `<main>` にページを表す class を付与する (`landing-page` / `list-page` / `show-page` / `new-page`)
- CSS はその class をネストの親にし、ページ固有のスタイルはすべてその中に閉じ込める
  - 例: `.list-page { .bookmark-item { ... } }`
- 全ページ共通として残すのは次のものだけ:
  - `:root` / `html` / `body` などのルート要素
  - `.xxx-page` より上位 (祖先) や兄弟に位置する shell (`.page-container` / `.page-header` / `main` / `.page-footer`)
- 複数ページで見た目が同じスタイル (フォーム・ナビリンク・要素リセット等) も、共有せず各ページへ複製する
  - **重複は許容する**。1ページの変更時に他ページへの影響を考えなくてよい局所性を優先するため
  - トレードオフとして横断的な一括変更はしづらくなる

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
