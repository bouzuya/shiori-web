#[derive(Clone, Debug)]
pub struct User {
    created_at: crate::entities::DateTime,
    id: crate::entities::UserId,
}

impl User {
    pub fn create(id: crate::entities::UserId) -> Self {
        Self {
            created_at: crate::entities::DateTime::now(),
            id,
        }
    }

    pub fn created_at(&self) -> crate::entities::DateTime {
        self.created_at
    }

    pub fn id(&self) -> &crate::entities::UserId {
        &self.id
    }

    pub fn new(created_at: crate::entities::DateTime, id: crate::entities::UserId) -> Self {
        Self { created_at, id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_create() -> anyhow::Result<()> {
        let user = User::create("user1".parse::<crate::entities::UserId>()?);
        assert_eq!(user.id().to_string(), "user1");
        Ok(())
    }

    #[test]
    fn test_user_create_has_created_at() -> anyhow::Result<()> {
        let before = crate::entities::DateTime::now();
        let user = User::create("user1".parse::<crate::entities::UserId>()?);
        let after = crate::entities::DateTime::now();
        assert!(user.created_at() >= before);
        assert!(user.created_at() <= after);
        Ok(())
    }
}
