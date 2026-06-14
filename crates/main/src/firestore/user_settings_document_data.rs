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

    pub(crate) fn from_user_settings(settings: &kernel::UserSettings) -> Self {
        Self {
            color_scheme: settings.color_scheme().to_string(),
        }
    }

    pub(crate) fn into_user_settings(
        self,
        user_id: kernel::UserId,
    ) -> anyhow::Result<kernel::UserSettings> {
        let color_scheme = self.color_scheme.parse::<kernel::ColorScheme>()?;
        Ok(kernel::UserSettings::new(
            color_scheme,
            user_id,
            kernel::UtcOffset::default(),
        ))
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

    #[test]
    fn test_from_user_settings() {
        let settings = kernel::UserSettings::new(
            kernel::ColorScheme::Dark,
            kernel::UserId::new(),
            kernel::UtcOffset::default(),
        );
        let data = UserSettingsDocumentData::from_user_settings(&settings);
        assert_eq!(data.color_scheme, "dark");
    }

    #[test]
    fn test_into_user_settings() -> anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "light".to_string(),
        };
        let settings = data.into_user_settings(user_id)?;
        assert_eq!(settings.color_scheme(), kernel::ColorScheme::Light);
        assert_eq!(settings.user_id(), user_id);
        Ok(())
    }
}
