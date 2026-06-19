use crate::DateTime;
use crate::GoogleUserId;
use crate::UserId;

#[derive(Clone, Debug)]
pub struct User {
    created_at: DateTime,
    google_user_id: GoogleUserId,
    id: UserId,
}

impl User {
    pub fn create(google_user_id: GoogleUserId) -> Self {
        Self {
            created_at: DateTime::now(),
            google_user_id,
            id: UserId::new(),
        }
    }

    pub fn created_at(&self) -> DateTime {
        self.created_at
    }

    pub fn google_user_id(&self) -> &GoogleUserId {
        &self.google_user_id
    }

    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn new(created_at: DateTime, google_user_id: GoogleUserId, id: UserId) -> Self {
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
    fn test_user_create() -> ::anyhow::Result<()> {
        let user = User::create("user1".parse::<GoogleUserId>()?);
        assert_eq!(user.google_user_id().to_string(), "user1");
        Ok(())
    }

    #[test]
    fn test_user_create_generates_user_id() -> ::anyhow::Result<()> {
        let user = User::create("user1".parse::<GoogleUserId>()?);
        let _id: UserId = user.id();
        Ok(())
    }

    #[test]
    fn test_user_create_has_created_at() -> ::anyhow::Result<()> {
        let before = DateTime::now();
        let user = User::create("user1".parse::<GoogleUserId>()?);
        let after = DateTime::now();
        assert!(user.created_at() >= before);
        assert!(user.created_at() <= after);
        Ok(())
    }
}
