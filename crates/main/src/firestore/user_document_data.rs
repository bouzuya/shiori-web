use kernel::DateTime;
use kernel::GoogleUserId;
use kernel::User;
use kernel::UserId;

/// Firestore の `users/{user_id}` ドキュメントの永続化形式。
#[derive(::serde::Deserialize, ::serde::Serialize)]
pub(crate) struct UserDocumentData {
    created_at: String,
    google_user_id: String,
    user_id: String,
}

impl UserDocumentData {
    pub(crate) fn from_user(user: &User) -> Self {
        Self {
            created_at: user.created_at().to_rfc3339(),
            google_user_id: user.google_user_id().to_string(),
            user_id: user.id().to_string(),
        }
    }

    pub(crate) fn into_user(self) -> ::anyhow::Result<User> {
        Ok(User::new(
            DateTime::from_rfc3339(&self.created_at)?,
            self.google_user_id.parse::<GoogleUserId>()?,
            self.user_id.parse::<UserId>()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_user_then_into_user_roundtrip() -> ::anyhow::Result<()> {
        let user = User::new(
            DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?,
            "google_user_id".parse::<GoogleUserId>()?,
            UserId::new(),
        );
        let restored = UserDocumentData::from_user(&user).into_user()?;
        assert_eq!(restored.created_at(), user.created_at());
        assert_eq!(
            restored.google_user_id().to_string(),
            user.google_user_id().to_string()
        );
        assert_eq!(restored.id(), user.id());
        Ok(())
    }
}
