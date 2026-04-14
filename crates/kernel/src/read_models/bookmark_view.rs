#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookmarkView {
    pub comment: String,
    pub created_at: String,
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub url: String,
    pub user_id: String,
}

#[cfg(test)]
impl BookmarkView {
    pub fn for_test() -> Self {
        Self {
            comment: crate::entities::Comment::for_test().to_string(),
            created_at: crate::entities::DateTime::now().to_rfc3339(),
            id: crate::entities::BookmarkId::new().to_string(),
            title: crate::entities::Title::for_test().to_string(),
            updated_at: crate::entities::DateTime::now().to_rfc3339(),
            url: crate::entities::Url::for_test().to_string(),
            user_id: crate::entities::UserId::new().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_view_fields() -> anyhow::Result<()> {
        let view = BookmarkView {
            comment: "memo".to_string(),
            created_at: "2024-01-01T00:00:00.000Z".to_string(),
            id: "01HX000000000000000000000A".to_string(),
            title: "Example".to_string(),
            updated_at: "2024-06-01T00:00:00.000Z".to_string(),
            url: "https://example.com/".to_string(),
            user_id: "01HX000000000000000000000U".to_string(),
        };
        assert_eq!(view.comment, "memo");
        assert_eq!(view.created_at, "2024-01-01T00:00:00.000Z");
        assert_eq!(view.id, "01HX000000000000000000000A");
        assert_eq!(view.title, "Example");
        assert_eq!(view.updated_at, "2024-06-01T00:00:00.000Z");
        assert_eq!(view.url, "https://example.com/");
        assert_eq!(view.user_id, "01HX000000000000000000000U");
        Ok(())
    }

    #[test]
    fn test_bookmark_view_clone_eq() -> anyhow::Result<()> {
        let view = BookmarkView {
            comment: "c".to_string(),
            created_at: "2024-01-01T00:00:00.000Z".to_string(),
            id: "id".to_string(),
            title: "t".to_string(),
            updated_at: "2024-01-01T00:00:00.000Z".to_string(),
            url: "https://example.com/".to_string(),
            user_id: "u".to_string(),
        };
        assert_eq!(view.clone(), view);
        Ok(())
    }
}
