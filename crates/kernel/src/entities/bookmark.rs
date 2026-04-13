#[derive(Clone, Debug)]
pub struct Bookmark {
    comment: crate::entities::Comment,
    created_at: crate::entities::DateTime,
    id: crate::entities::BookmarkId,
    title: crate::entities::Title,
    updated_at: crate::entities::DateTime,
    url: crate::entities::Url,
    user_id: crate::entities::UserId,
}

impl Bookmark {
    pub fn create(
        user_id: crate::entities::UserId,
        url: crate::entities::Url,
        title: crate::entities::Title,
        comment: crate::entities::Comment,
    ) -> Self {
        let now = crate::entities::DateTime::now();
        Self {
            comment,
            created_at: now,
            id: crate::entities::BookmarkId::new(),
            title,
            updated_at: now,
            url,
            user_id,
        }
    }

    pub fn new(
        comment: crate::entities::Comment,
        created_at: crate::entities::DateTime,
        id: crate::entities::BookmarkId,
        title: crate::entities::Title,
        updated_at: crate::entities::DateTime,
        url: crate::entities::Url,
        user_id: crate::entities::UserId,
    ) -> Self {
        Self {
            comment,
            created_at,
            id,
            title,
            updated_at,
            url,
            user_id,
        }
    }

    pub fn comment(&self) -> &crate::entities::Comment {
        &self.comment
    }

    pub fn created_at(&self) -> crate::entities::DateTime {
        self.created_at
    }

    pub fn id(&self) -> crate::entities::BookmarkId {
        self.id
    }

    pub fn title(&self) -> &crate::entities::Title {
        &self.title
    }

    pub fn updated_at(&self) -> crate::entities::DateTime {
        self.updated_at
    }

    pub fn url(&self) -> &crate::entities::Url {
        &self.url
    }

    pub fn user_id(&self) -> crate::entities::UserId {
        self.user_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_url() -> anyhow::Result<crate::entities::Url> {
        Ok("https://example.com".parse()?)
    }

    fn sample_title() -> anyhow::Result<crate::entities::Title> {
        Ok("Example".parse()?)
    }

    fn sample_comment() -> anyhow::Result<crate::entities::Comment> {
        Ok("my comment".parse()?)
    }

    #[test]
    fn test_bookmark_create_generates_id() -> anyhow::Result<()> {
        let user_id = crate::entities::UserId::new();
        let b = Bookmark::create(user_id, sample_url()?, sample_title()?, sample_comment()?);
        let _id: crate::entities::BookmarkId = b.id();
        Ok(())
    }

    #[test]
    fn test_bookmark_create_unique_ids() -> anyhow::Result<()> {
        let user_id = crate::entities::UserId::new();
        let b1 = Bookmark::create(user_id, sample_url()?, sample_title()?, sample_comment()?);
        let b2 = Bookmark::create(user_id, sample_url()?, sample_title()?, sample_comment()?);
        assert_ne!(b1.id(), b2.id());
        Ok(())
    }

    #[test]
    fn test_bookmark_create_stores_fields() -> anyhow::Result<()> {
        let user_id = crate::entities::UserId::new();
        let b = Bookmark::create(user_id, sample_url()?, sample_title()?, sample_comment()?);
        assert_eq!(b.user_id(), user_id);
        assert_eq!(b.url().to_string(), "https://example.com/");
        assert_eq!(b.title().to_string(), "Example");
        assert_eq!(b.comment().to_string(), "my comment");
        Ok(())
    }

    #[test]
    fn test_bookmark_create_has_created_at() -> anyhow::Result<()> {
        let before = crate::entities::DateTime::now();
        let user_id = crate::entities::UserId::new();
        let b = Bookmark::create(user_id, sample_url()?, sample_title()?, sample_comment()?);
        let after = crate::entities::DateTime::now();
        assert!(b.created_at() >= before);
        assert!(b.created_at() <= after);
        Ok(())
    }

    #[test]
    fn test_bookmark_create_updated_at_equals_created_at() -> anyhow::Result<()> {
        let user_id = crate::entities::UserId::new();
        let b = Bookmark::create(user_id, sample_url()?, sample_title()?, sample_comment()?);
        assert_eq!(b.created_at(), b.updated_at());
        Ok(())
    }

    #[test]
    fn test_bookmark_new_roundtrip() -> anyhow::Result<()> {
        let id = crate::entities::BookmarkId::new();
        let user_id = crate::entities::UserId::new();
        let created_at = crate::entities::DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?;
        let updated_at = crate::entities::DateTime::from_rfc3339("2024-06-01T00:00:00.000Z")?;
        let b = Bookmark::new(
            sample_comment()?,
            created_at,
            id,
            sample_title()?,
            updated_at,
            sample_url()?,
            user_id,
        );
        assert_eq!(b.id(), id);
        assert_eq!(b.user_id(), user_id);
        assert_eq!(b.created_at(), created_at);
        assert_eq!(b.updated_at(), updated_at);
        Ok(())
    }
}
