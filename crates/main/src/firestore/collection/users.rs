use crate::firestore::FirestoreCollection;

/// Firestore の `users` コレクション。
pub(crate) struct Users;

impl FirestoreCollection for Users {
    type DocumentId = kernel::UserId;
    type ParentDocumentId = ();
    type Schema = crate::firestore::UserDocumentData;

    fn collection_path(_parent: &Self::ParentDocumentId) -> String {
        "users".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_path() -> anyhow::Result<()> {
        assert_eq!(Users::collection_path(&()), "users");
        Ok(())
    }

    #[test]
    fn test_document_path() -> anyhow::Result<()> {
        let user_id = "01234567-89ab-cdef-0123-456789abcdef".parse::<kernel::UserId>()?;
        assert_eq!(
            Users::document_path(&(), &user_id),
            "users/01234567-89ab-cdef-0123-456789abcdef"
        );
        Ok(())
    }
}
