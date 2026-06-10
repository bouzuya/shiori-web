/// Firestore の `users/{user_id}/bookmarks/{bookmark_id}` ドキュメントの永続化形式。
///
/// `user_id` はドキュメントのパスから復元できるため保存しない。
/// `deleted_at` も保存しない (削除は論理削除フラグではなくドキュメントの物理削除で表現するため)。
#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct BookmarkDocumentData {
    bookmark_id: String,
    comment: String,
    created_at: String,
    title: String,
    updated_at: String,
    url: String,
}

impl BookmarkDocumentData {
    pub(crate) fn from_bookmark(bookmark: &kernel::Bookmark) -> Self {
        Self {
            bookmark_id: bookmark.id().to_string(),
            comment: bookmark.comment().to_string(),
            created_at: bookmark.created_at().to_rfc3339(),
            title: bookmark.title().to_string(),
            updated_at: bookmark.updated_at().to_rfc3339(),
            url: bookmark.url().to_string(),
        }
    }

    pub(crate) fn into_bookmark(self, user_id: kernel::UserId) -> anyhow::Result<kernel::Bookmark> {
        Ok(kernel::Bookmark::new(
            self.comment.parse::<kernel::Comment>()?,
            kernel::DateTime::from_rfc3339(&self.created_at)?,
            None,
            self.bookmark_id.parse::<kernel::BookmarkId>()?,
            self.title.parse::<kernel::Title>()?,
            kernel::DateTime::from_rfc3339(&self.updated_at)?,
            self.url.parse::<kernel::Url>()?,
            user_id,
        ))
    }

    pub(crate) fn into_bookmark_view(self, user_id: kernel::UserId) -> kernel::BookmarkView {
        kernel::BookmarkView {
            comment: self.comment,
            created_at: self.created_at,
            id: self.bookmark_id,
            title: self.title,
            updated_at: self.updated_at,
            url: self.url,
            user_id: user_id.to_string(),
        }
    }

    pub(crate) fn updated_at(&self) -> &str {
        &self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bookmark_then_into_bookmark_roundtrip() -> anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let bookmark = kernel::Bookmark::new(
            "comment".parse::<kernel::Comment>()?,
            kernel::DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?,
            None,
            kernel::BookmarkId::new(),
            "title".parse::<kernel::Title>()?,
            kernel::DateTime::from_rfc3339("2024-06-01T00:00:00.000Z")?,
            "https://example.com/".parse::<kernel::Url>()?,
            user_id,
        );
        let restored = BookmarkDocumentData::from_bookmark(&bookmark).into_bookmark(user_id)?;
        assert_eq!(restored.id(), bookmark.id());
        assert_eq!(restored.comment().to_string(), "comment");
        assert_eq!(restored.created_at(), bookmark.created_at());
        assert_eq!(restored.title().to_string(), "title");
        assert_eq!(restored.updated_at(), bookmark.updated_at());
        assert_eq!(restored.url().to_string(), "https://example.com/");
        assert_eq!(restored.user_id(), user_id);
        // deleted_at は永続化されないため復元後は常に None
        assert_eq!(restored.deleted_at(), None);
        Ok(())
    }

    #[test]
    fn test_into_bookmark_view() -> anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let data = BookmarkDocumentData {
            bookmark_id: "bid".to_string(),
            comment: "c".to_string(),
            created_at: "2024-01-01T00:00:00.000Z".to_string(),
            title: "t".to_string(),
            updated_at: "2024-06-01T00:00:00.000Z".to_string(),
            url: "https://example.com/".to_string(),
        };
        let view = data.into_bookmark_view(user_id);
        assert_eq!(view.comment, "c");
        assert_eq!(view.created_at, "2024-01-01T00:00:00.000Z");
        assert_eq!(view.id, "bid");
        assert_eq!(view.title, "t");
        assert_eq!(view.updated_at, "2024-06-01T00:00:00.000Z");
        assert_eq!(view.url, "https://example.com/");
        assert_eq!(view.user_id, user_id.to_string());
        Ok(())
    }

    #[test]
    fn test_updated_at_accessor() -> anyhow::Result<()> {
        let data = BookmarkDocumentData {
            bookmark_id: "bid".to_string(),
            comment: "c".to_string(),
            created_at: "2024-01-01T00:00:00.000Z".to_string(),
            title: "t".to_string(),
            updated_at: "2024-06-01T00:00:00.000Z".to_string(),
            url: "https://example.com/".to_string(),
        };
        assert_eq!(data.updated_at(), "2024-06-01T00:00:00.000Z");
        Ok(())
    }
}
