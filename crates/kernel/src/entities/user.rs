#[derive(Clone, Debug)]
pub struct User {
    created_at: crate::entities::DateTime,
    google_user_id: crate::entities::GoogleUserId,
    id: crate::entities::UserId,
}

impl User {
    pub fn create(google_user_id: crate::entities::GoogleUserId) -> Self {
        Self {
            created_at: crate::entities::DateTime::now(),
            google_user_id,
            id: crate::entities::UserId::new(),
        }
    }

    pub fn created_at(&self) -> crate::entities::DateTime {
        self.created_at
    }

    pub fn google_user_id(&self) -> &crate::entities::GoogleUserId {
        &self.google_user_id
    }

    pub fn id(&self) -> crate::entities::UserId {
        self.id
    }

    pub fn new(
        created_at: crate::entities::DateTime,
        google_user_id: crate::entities::GoogleUserId,
        id: crate::entities::UserId,
    ) -> Self {
        Self {
            created_at,
            google_user_id,
            id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_create() -> anyhow::Result<()> {
        let user = User::create("user1".parse::<crate::entities::GoogleUserId>()?);
        assert_eq!(user.google_user_id().to_string(), "user1");
        Ok(())
    }

    #[test]
    fn test_user_create_generates_user_id() -> anyhow::Result<()> {
        let user = User::create("user1".parse::<crate::entities::GoogleUserId>()?);
        let _id: crate::entities::UserId = user.id();
        Ok(())
    }

    #[test]
    fn test_user_create_has_created_at() -> anyhow::Result<()> {
        let before = crate::entities::DateTime::now();
        let user = User::create("user1".parse::<crate::entities::GoogleUserId>()?);
        let after = crate::entities::DateTime::now();
        assert!(user.created_at() >= before);
        assert!(user.created_at() <= after);
        Ok(())
    }
}
