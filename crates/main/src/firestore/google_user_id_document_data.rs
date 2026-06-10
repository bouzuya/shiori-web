/// Firestore の `google_user_ids/{google_user_id}` ドキュメントの永続化形式。
///
/// `google_user_id` から `user_id` を引くためのインデックス用ドキュメント。
/// `google_user_id` 自体はドキュメントのパスから復元できるため保存しない。
#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct GoogleUserIdDocumentData {
    user_id: String,
}

impl GoogleUserIdDocumentData {
    pub(crate) fn from_user(user: &crate::model::User) -> Self {
        Self {
            user_id: user.id().to_string(),
        }
    }

    pub(crate) fn into_user_id(self) -> anyhow::Result<crate::model::UserId> {
        self.user_id.parse::<crate::model::UserId>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_user_then_into_user_id_roundtrip() -> anyhow::Result<()> {
        let user = crate::model::User::new(
            crate::model::DateTime::from_rfc3339("2024-01-01T00:00:00.000Z")?,
            "google_user_id".parse::<crate::model::GoogleUserId>()?,
            crate::model::UserId::new(),
        );
        let user_id = GoogleUserIdDocumentData::from_user(&user).into_user_id()?;
        assert_eq!(user_id, user.id());
        Ok(())
    }
}
