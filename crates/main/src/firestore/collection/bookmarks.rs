use crate::BookmarkDocumentData;
use crate::FirestoreCollection;
use crate::UsersCollection;
use kernel::BookmarkId;
use kernel::UserId;

/// Firestore の `users/{user_id}/bookmarks` サブコレクション。
#[derive(Clone)]
pub(crate) struct BookmarksCollection;

impl FirestoreCollection for BookmarksCollection {
    type DocumentId = BookmarkId;
    type ParentDocumentId = UserId;
    type Schema = BookmarkDocumentData;

    fn collection_path(parent: &Self::ParentDocumentId) -> String {
        // 親ドキュメント (`users/{user_id}`) のパスは `UsersCollection` に委譲し、
        // `users/` プレフィックスをここで重複して持たない。
        format!("{}/bookmarks", UsersCollection::document_path(&(), parent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_path() -> ::anyhow::Result<()> {
        let user_id = "01234567-89ab-cdef-0123-456789abcdef".parse::<UserId>()?;
        assert_eq!(
            BookmarksCollection::collection_path(&user_id),
            "users/01234567-89ab-cdef-0123-456789abcdef/bookmarks"
        );
        Ok(())
    }

    #[test]
    fn test_document_path() -> ::anyhow::Result<()> {
        let user_id = "01234567-89ab-cdef-0123-456789abcdef".parse::<UserId>()?;
        let bookmark_id = "fedcba98-7654-3210-fedc-ba9876543210".parse::<BookmarkId>()?;
        assert_eq!(
            BookmarksCollection::document_path(&user_id, &bookmark_id),
            "users/01234567-89ab-cdef-0123-456789abcdef/bookmarks/fedcba98-7654-3210-fedc-ba9876543210"
        );
        Ok(())
    }
}
