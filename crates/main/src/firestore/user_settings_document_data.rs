/// Firestore の `user_settings/{user_id}` ドキュメントの永続化形式。
///
/// `user_id` はドキュメントのパスから復元できるため保存しない。
#[derive(serde::Deserialize, serde::Serialize)]
pub(crate) struct UserSettingsDocumentData {
    color_scheme: String,
    utc_offset: String,
}

impl UserSettingsDocumentData {
    pub(crate) fn into_user_settings_view(
        self,
        user_id: kernel::UserId,
    ) -> anyhow::Result<kernel::UserSettingsView> {
        // 不正な値を弾き、正規化した文字列を保持する。
        let color_scheme = self.color_scheme.parse::<kernel::ColorScheme>()?;
        let utc_offset = self.utc_offset.parse::<kernel::UtcOffset>()?;
        Ok(kernel::UserSettingsView {
            color_scheme: color_scheme.to_string(),
            // FIXME: share_url の永続化は後続コミットで実装する (今は None 固定)
            share_url: None,
            user_id: user_id.to_string(),
            utc_offset: utc_offset.to_string(),
        })
    }

    pub(crate) fn from_user_settings(settings: &kernel::UserSettings) -> Self {
        Self {
            color_scheme: settings.color_scheme().to_string(),
            utc_offset: settings.utc_offset().to_string(),
        }
    }

    pub(crate) fn into_user_settings(
        self,
        user_id: kernel::UserId,
    ) -> anyhow::Result<kernel::UserSettings> {
        let color_scheme = self.color_scheme.parse::<kernel::ColorScheme>()?;
        let utc_offset = self.utc_offset.parse::<kernel::UtcOffset>()?;
        Ok(kernel::UserSettings::new(
            color_scheme,
            // FIXME: share_url の永続化は後続コミットで実装する (今は None 固定)
            None,
            user_id,
            utc_offset,
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
            utc_offset: "+09:00".to_string(),
        };
        let view = data.into_user_settings_view(user_id)?;
        assert_eq!(view.color_scheme, "dark");
        assert_eq!(view.user_id, user_id.to_string());
        assert_eq!(view.utc_offset, "+09:00");
        Ok(())
    }

    #[test]
    fn test_into_user_settings_view_rejects_invalid_color_scheme() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "auto".to_string(),
            utc_offset: "+00:00".to_string(),
        };
        assert!(data.into_user_settings_view(user_id).is_err());
    }

    #[test]
    fn test_into_user_settings_view_rejects_invalid_utc_offset() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "dark".to_string(),
            utc_offset: "invalid".to_string(),
        };
        assert!(data.into_user_settings_view(user_id).is_err());
    }

    #[test]
    fn test_from_user_settings() -> anyhow::Result<()> {
        let settings = kernel::UserSettings::new(
            kernel::ColorScheme::Dark,
            None,
            kernel::UserId::new(),
            kernel::UtcOffset::new(540)?,
        );
        let data = UserSettingsDocumentData::from_user_settings(&settings);
        assert_eq!(data.color_scheme, "dark");
        assert_eq!(data.utc_offset, "+09:00");
        Ok(())
    }

    #[test]
    fn test_into_user_settings() -> anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "light".to_string(),
            utc_offset: "-05:00".to_string(),
        };
        let settings = data.into_user_settings(user_id)?;
        assert_eq!(settings.color_scheme(), kernel::ColorScheme::Light);
        assert_eq!(settings.user_id(), user_id);
        assert_eq!(settings.utc_offset(), kernel::UtcOffset::new(-300)?);
        Ok(())
    }

    #[test]
    fn test_into_user_settings_rejects_invalid_utc_offset() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "light".to_string(),
            utc_offset: "invalid".to_string(),
        };
        assert!(data.into_user_settings(user_id).is_err());
    }
}
