use crate::FirestoreCollection;
use crate::UserSettingsDocumentData;

/// Firestore の `user_settings` コレクション。
#[derive(Clone)]
pub(crate) struct UserSettingsCollection;

impl FirestoreCollection for UserSettingsCollection {
    type DocumentId = kernel::UserId;
    type ParentDocumentId = ();
    type Schema = UserSettingsDocumentData;

    fn collection_path(_parent: &Self::ParentDocumentId) -> String {
        "user_settings".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_path() -> anyhow::Result<()> {
        assert_eq!(
            UserSettingsCollection::collection_path(&()),
            "user_settings"
        );
        Ok(())
    }

    #[test]
    fn test_document_path() -> anyhow::Result<()> {
        let user_id = "01234567-89ab-cdef-0123-456789abcdef".parse::<kernel::UserId>()?;
        assert_eq!(
            UserSettingsCollection::document_path(&(), &user_id),
            "user_settings/01234567-89ab-cdef-0123-456789abcdef"
        );
        Ok(())
    }
}
