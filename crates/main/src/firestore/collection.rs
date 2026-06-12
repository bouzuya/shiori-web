mod bookmarks;
mod google_user_ids;
mod user_settings;
mod users;

pub(crate) use self::bookmarks::BookmarksCollection;
pub(crate) use self::google_user_ids::GoogleUserIdsCollection;
pub(crate) use self::user_settings::UserSettingsCollection;
pub(crate) use self::users::UsersCollection;

/// Firestore のコレクション (とその中のドキュメント) のパス構成を型ごとに表す。
///
/// ```text
/// users/{user_id}
/// users/{user_id}/bookmarks/{bookmark_id}
/// google_user_ids/{hex(google_user_id)}
/// ```
///
/// パス文字列を各リポジトリに直書きすると構成知識が散らばるため、
/// 各コレクション型だけがそのパスの形を知っている状態を保つこと。
/// `document_path` は `collection_path` を基点に組み立てるため、
/// 親プレフィックスを重複して持たない。
///
/// このプロジェクトでは Firestore のコレクションは 2 層まで
/// (トップレベル / その直下のサブコレクション) しか設けない前提を置く。
/// そのため親ドキュメントは常にトップレベルの 1 ドキュメントで、
/// `ParentDocumentId` という単一の ID で一意に指せる。
/// 3 層以上を扱うことになった場合はこの前提ごと再設計すること。
pub(crate) trait FirestoreCollection {
    /// コレクション内のドキュメントを指す ID
    type DocumentId: std::fmt::Display;
    /// このコレクションがぶら下がる親ドキュメントを指す ID。トップレベルは `()`。
    type ParentDocumentId;
    /// このコレクションのドキュメントの永続化形式 (スキーマ)。
    type Schema: serde::de::DeserializeOwned + serde::Serialize;

    fn collection_path(parent: &Self::ParentDocumentId) -> String;

    /// `DocumentId` を Firestore のドキュメント ID 文字列へ変換する。
    /// 既定では `to_string`。エンコードが必要なコレクションだけ上書きする。
    fn document_id_segment(id: &Self::DocumentId) -> String {
        id.to_string()
    }

    fn document_path(parent: &Self::ParentDocumentId, id: &Self::DocumentId) -> String {
        format!(
            "{}/{}",
            Self::collection_path(parent),
            Self::document_id_segment(id)
        )
    }
}
