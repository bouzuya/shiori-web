use crate::BookmarkId;
use crate::Comment;
use crate::DateTime;
use crate::Title;
use crate::Url;
use crate::UserId;

#[derive(Clone, Debug)]
pub struct Bookmark {
    comment: Comment,
    created_at: DateTime,
    deleted_at: Option<DateTime>,
    id: BookmarkId,
    title: Title,
    updated_at: DateTime,
    url: Url,
    user_id: UserId,
}

impl Bookmark {
    pub fn create(user_id: UserId, url: Url, title: Title, comment: Comment) -> Self {
        let now = DateTime::now();
        Self {
            comment,
            created_at: now,
            deleted_at: None,
            id: BookmarkId::new(),
            title,
            updated_at: now,
            url,
            user_id,
        }
    }

    pub fn new(
        comment: Comment,
        created_at: DateTime,
        deleted_at: Option<DateTime>,
        id: BookmarkId,
        title: Title,
        updated_at: DateTime,
        url: Url,
        user_id: UserId,
    ) -> Self {
        Self {
            comment,
            created_at,
            deleted_at,
            id,
            title,
            updated_at,
            url,
            user_id,
        }
    }

    pub fn comment(&self) -> &Comment {
        &self.comment
    }

    pub fn created_at(&self) -> DateTime {
        self.created_at
    }

    pub fn deleted_at(&self) -> Option<DateTime> {
        self.deleted_at
    }

    pub fn id(&self) -> BookmarkId {
        self.id
    }

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn updated_at(&self) -> DateTime {
        self.updated_at
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }
}

#[cfg(test)]
impl Bookmark {
    pub fn for_test() -> Self {
        Self {
            comment: Comment::for_test(),
            created_at: DateTime::now(),
            deleted_at: None,
            id: BookmarkId::new(),
            title: Title::for_test(),
            updated_at: DateTime::now(),
            url: Url::for_test(),
            user_id: UserId::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bookmark_create_generates_id() -> anyhow::Result<()> {
        let user_id = UserId::new();
        let b = Bookmark::create(
            user_id,
            Url::for_test(),
            Title::for_test(),
            Comment::for_test(),
        );
        let _id: BookmarkId = b.id();
        Ok(())
    }

    #[test]
    fn test_bookmark_create_unique_ids() -> anyhow::Result<()> {
        let user_id = UserId::new();
        let b1 = Bookmark::create(
            user_id,
            Url::for_test(),
            Title::for_test(),
            Comment::for_test(),
        );
        let b2 = Bookmark::create(
            user_id,
            Url::for_test(),
            Title::for_test(),
            Comment::for_test(),
        );
        assert_ne!(b1.id(), b2.id());
        Ok(())
    }

    #[test]
    fn test_bookmark_create_stores_fields() -> anyhow::Result<()> {
        let user_id = UserId::new();
        let b = Bookmark::create(
            user_id,
            Url::for_test(),
            Title::for_test(),
            Comment::for_test(),
        );
        assert_eq!(b.user_id(), user_id);
        assert!(b.url().to_string().starts_with("https://example.com/"));
        assert!(b.title().to_string().len() <= 255);
        assert!(b.comment().to_string().len() <= 255);
        Ok(())
    }

    #[test]
    fn test_bookmark_create_has_created_at() -> anyhow::Result<()> {
        let before = DateTime::now();
        let user_id = UserId::new();
        let b = Bookmark::create(
            user_id,
            Url::for_test(),
            Title::for_test(),
            Comment::for_test(),
        );
        let after = DateTime::now();
        assert!(b.created_at() >= before);
        assert!(b.created_at() <= after);
        Ok(())
    }

    #[test]
    fn test_bookmark_create_updated_at_equals_created_at() -> anyhow::Result<()> {
        let user_id = UserId::new();
        let b = Bookmark::create(
            user_id,
            Url::for_test(),
            Title::for_test(),
            Comment::for_test(),
        );
        assert_eq!(b.created_at(), b.updated_at());
        Ok(())
    }

    #[test]
    fn test_bookmark_new_roundtrip() -> anyhow::Result<()> {
        let id = BookmarkId::new();
        let user_id = UserId::new();
        let created_at = DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?;
        let updated_at = DateTime::from_rfc3339("2024-06-01T00:00:00.000Z")?;
        let b = Bookmark::new(
            Comment::for_test(),
            created_at,
            None,
            id,
            Title::for_test(),
            updated_at,
            Url::for_test(),
            user_id,
        );
        assert_eq!(b.id(), id);
        assert_eq!(b.user_id(), user_id);
        assert_eq!(b.created_at(), created_at);
        assert_eq!(b.deleted_at(), None);
        assert_eq!(b.updated_at(), updated_at);
        Ok(())
    }

    #[test]
    fn test_bookmark_new_with_deleted_at() -> anyhow::Result<()> {
        let deleted_at = DateTime::from_rfc3339("2024-06-01T00:00:00.000Z")?;
        let b = Bookmark::new(
            Comment::for_test(),
            DateTime::now(),
            Some(deleted_at),
            BookmarkId::new(),
            Title::for_test(),
            DateTime::now(),
            Url::for_test(),
            UserId::new(),
        );
        assert_eq!(b.deleted_at(), Some(deleted_at));
        Ok(())
    }

    #[test]
    fn test_bookmark_create_deleted_at_is_none() -> anyhow::Result<()> {
        let user_id = UserId::new();
        let b = Bookmark::create(
            user_id,
            Url::for_test(),
            Title::for_test(),
            Comment::for_test(),
        );
        assert_eq!(b.deleted_at(), None);
        Ok(())
    }
}
