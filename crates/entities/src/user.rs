#[derive(Clone, Debug)]
pub struct User {
    created_at: crate::DateTime,
    id: crate::UserId,
}

impl User {
    pub fn create(id: crate::UserId) -> Self {
        Self {
            created_at: crate::DateTime::now(),
            id,
        }
    }

    pub fn created_at(&self) -> crate::DateTime {
        self.created_at
    }

    pub fn id(&self) -> &crate::UserId {
        &self.id
    }

    pub fn new(created_at: crate::DateTime, id: crate::UserId) -> Self {
        Self { created_at, id }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_create() -> anyhow::Result<()> {
        let user = User::create("user1".parse::<crate::UserId>()?);
        assert_eq!(user.id().to_string(), "user1");
        Ok(())
    }

    #[test]
    fn test_user_create_has_created_at() -> anyhow::Result<()> {
        let before = crate::DateTime::now();
        let user = User::create("user1".parse::<crate::UserId>()?);
        let after = crate::DateTime::now();
        assert!(user.created_at() >= before);
        assert!(user.created_at() <= after);
        Ok(())
    }
}
