//! Firestore のコレクション / ドキュメントのパス構成を一元管理する。
//!
//! ```text
//! users/{user_id}
//! users/{user_id}/bookmarks/{bookmark_id}
//! google_user_ids/{hex(google_user_id)}
//! ```
//!
//! パス文字列を各リポジトリに直書きすると構成知識が散らばるため、
//! ここのビルダ関数だけがパスの形を知っている状態を保つこと。

pub(crate) fn bookmark_collection(user_id: kernel::UserId) -> String {
    format!("users/{user_id}/bookmarks")
}

pub(crate) fn bookmark_document(
    user_id: kernel::UserId,
    bookmark_id: kernel::BookmarkId,
) -> String {
    format!("users/{user_id}/bookmarks/{bookmark_id}")
}

pub(crate) fn google_user_id_document(google_user_id: &kernel::GoogleUserId) -> String {
    let document_id = google_user_id_to_document_id(&google_user_id.to_string());
    format!("google_user_ids/{document_id}")
}

pub(crate) fn user_document(user_id: kernel::UserId) -> String {
    format!("users/{user_id}")
}

/// `google_user_id` は大文字小文字を区別する任意の ASCII 文字列だが、
/// Firestore のドキュメント ID として安全に使うためバイト列の 16 進数に変換する。
fn google_user_id_to_document_id(id: &str) -> String {
    id.bytes().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_collection() -> anyhow::Result<()> {
        let user_id = "01234567-89ab-cdef-0123-456789abcdef".parse::<kernel::UserId>()?;
        assert_eq!(
            bookmark_collection(user_id),
            "users/01234567-89ab-cdef-0123-456789abcdef/bookmarks"
        );
        Ok(())
    }

    #[test]
    fn test_bookmark_document() -> anyhow::Result<()> {
        let user_id = "01234567-89ab-cdef-0123-456789abcdef".parse::<kernel::UserId>()?;
        let bookmark_id = "fedcba98-7654-3210-fedc-ba9876543210".parse::<kernel::BookmarkId>()?;
        assert_eq!(
            bookmark_document(user_id, bookmark_id),
            "users/01234567-89ab-cdef-0123-456789abcdef/bookmarks/fedcba98-7654-3210-fedc-ba9876543210"
        );
        Ok(())
    }

    #[test]
    fn test_google_user_id_document() -> anyhow::Result<()> {
        // a=0x61, b=0x62, c=0x63, 1=0x31, 2=0x32, 3=0x33
        let google_user_id = "abc123".parse::<kernel::GoogleUserId>()?;
        assert_eq!(
            google_user_id_document(&google_user_id),
            "google_user_ids/616263313233"
        );
        Ok(())
    }

    #[test]
    fn test_user_document() -> anyhow::Result<()> {
        let user_id = "01234567-89ab-cdef-0123-456789abcdef".parse::<kernel::UserId>()?;
        assert_eq!(
            user_document(user_id),
            "users/01234567-89ab-cdef-0123-456789abcdef"
        );
        Ok(())
    }

    #[test]
    fn test_google_user_id_to_document_id() -> anyhow::Result<()> {
        assert_eq!(google_user_id_to_document_id("abc123"), "616263313233");
        Ok(())
    }
}
