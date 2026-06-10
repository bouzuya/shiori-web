use crate::firestore::FirestoreCollection;

/// Firestore の `google_user_ids` コレクション。
///
/// `google_user_id` から `user_id` を引くためのインデックス用コレクション。
pub(crate) struct GoogleUserIds;

impl FirestoreCollection for GoogleUserIds {
    type DocumentId = kernel::GoogleUserId;
    type ParentDocumentId = ();
    type Schema = crate::firestore::GoogleUserIdDocumentData;

    fn collection_path(_parent: &Self::ParentDocumentId) -> String {
        "google_user_ids".to_string()
    }

    /// `google_user_id` は大文字小文字を区別する任意の ASCII 文字列だが、
    /// Firestore のドキュメント ID として安全に使うためバイト列の 16 進数に変換する。
    fn document_id_segment(id: &Self::DocumentId) -> String {
        id.to_string().bytes().map(|b| format!("{b:02x}")).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_path() -> anyhow::Result<()> {
        assert_eq!(GoogleUserIds::collection_path(&()), "google_user_ids");
        Ok(())
    }

    #[test]
    fn test_document_path() -> anyhow::Result<()> {
        // a=0x61, b=0x62, c=0x63, 1=0x31, 2=0x32, 3=0x33
        let google_user_id = "abc123".parse::<kernel::GoogleUserId>()?;
        assert_eq!(
            GoogleUserIds::document_path(&(), &google_user_id),
            "google_user_ids/616263313233"
        );
        Ok(())
    }
}
