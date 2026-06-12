/// Firestore の `user_settings/{user_id}` ドキュメントの永続化形式。
///
/// `user_id` はドキュメントのパスから復元できるため保存しない。
#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct UserSettingsDocumentData {
    color_scheme: String,
}

impl UserSettingsDocumentData {
    pub(crate) fn into_user_settings_view(
        self,
        user_id: kernel::UserId,
    ) -> anyhow::Result<kernel::UserSettingsView> {
        // 不正な値を弾き、正規化した文字列を保持する。
        let color_scheme = self.color_scheme.parse::<kernel::ColorScheme>()?;
        Ok(kernel::UserSettingsView {
            color_scheme: color_scheme.to_string(),
            user_id: user_id.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_user_settings_view() -> anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "dark".to_string(),
        };
        let view = data.into_user_settings_view(user_id)?;
        assert_eq!(view.color_scheme, "dark");
        assert_eq!(view.user_id, user_id.to_string());
        Ok(())
    }

    #[test]
    fn test_into_user_settings_view_rejects_invalid_color_scheme() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "auto".to_string(),
        };
        assert!(data.into_user_settings_view(user_id).is_err());
    }
}
