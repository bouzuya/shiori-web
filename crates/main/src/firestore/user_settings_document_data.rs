/// Firestore の `user_settings/{user_id}` ドキュメントの永続化形式。
///
/// `user_id` はドキュメントのパスから復元できるため保存しない。
#[derive(::serde::Deserialize, ::serde::Serialize)]
pub(crate) struct UserSettingsDocumentData {
    color_scheme: String,
    share_url: Option<String>,
    utc_offset: String,
}

impl UserSettingsDocumentData {
    pub(crate) fn into_user_settings_view(
        self,
        user_id: kernel::UserId,
    ) -> ::anyhow::Result<kernel::UserSettingsView> {
        // 不正な値を弾き、正規化した文字列を保持する。
        let color_scheme = self.color_scheme.parse::<kernel::ColorScheme>()?;
        let share_url = self
            .share_url
            .map(|s| s.parse::<kernel::ShareUrl>())
            .transpose()?;
        let utc_offset = self.utc_offset.parse::<kernel::UtcOffset>()?;
        Ok(kernel::UserSettingsView {
            color_scheme: color_scheme.to_string(),
            share_url: share_url.map(|s| s.to_string()),
            user_id: user_id.to_string(),
            utc_offset: utc_offset.to_string(),
        })
    }

    pub(crate) fn from_user_settings(settings: &kernel::UserSettings) -> Self {
        Self {
            color_scheme: settings.color_scheme().to_string(),
            share_url: settings.share_url().map(|s| s.to_string()),
            utc_offset: settings.utc_offset().to_string(),
        }
    }

    pub(crate) fn into_user_settings(
        self,
        user_id: kernel::UserId,
    ) -> ::anyhow::Result<kernel::UserSettings> {
        let color_scheme = self.color_scheme.parse::<kernel::ColorScheme>()?;
        let share_url = self
            .share_url
            .map(|s| s.parse::<kernel::ShareUrl>())
            .transpose()?;
        let utc_offset = self.utc_offset.parse::<kernel::UtcOffset>()?;
        Ok(kernel::UserSettings::new(
            color_scheme,
            share_url,
            user_id,
            utc_offset,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_user_settings_view() -> ::anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "dark".to_string(),
            share_url: Some("https://example.com/?u={{url}}".to_string()),
            utc_offset: "+09:00".to_string(),
        };
        let view = data.into_user_settings_view(user_id)?;
        assert_eq!(view.color_scheme, "dark");
        assert_eq!(
            view.share_url.as_deref(),
            Some("https://example.com/?u={{url}}")
        );
        assert_eq!(view.user_id, user_id.to_string());
        assert_eq!(view.utc_offset, "+09:00");
        Ok(())
    }

    #[test]
    fn test_into_user_settings_view_without_share_url() -> ::anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "dark".to_string(),
            share_url: None,
            utc_offset: "+09:00".to_string(),
        };
        let view = data.into_user_settings_view(user_id)?;
        assert_eq!(view.share_url, None);
        Ok(())
    }

    #[test]
    fn test_deserialize_without_share_url_defaults_to_none() -> ::anyhow::Result<()> {
        // 既存ドキュメント (share_url フィールドなし) との後方互換。
        let data = ::serde_json::from_str::<UserSettingsDocumentData>(
            r#"{"color_scheme":"dark","utc_offset":"+09:00"}"#,
        )?;
        assert_eq!(data.share_url, None);
        Ok(())
    }

    #[test]
    fn test_into_user_settings_view_rejects_invalid_color_scheme() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "auto".to_string(),
            share_url: None,
            utc_offset: "+00:00".to_string(),
        };
        assert!(data.into_user_settings_view(user_id).is_err());
    }

    #[test]
    fn test_into_user_settings_view_rejects_invalid_share_url() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "dark".to_string(),
            share_url: Some("not a url".to_string()),
            utc_offset: "+00:00".to_string(),
        };
        assert!(data.into_user_settings_view(user_id).is_err());
    }

    #[test]
    fn test_into_user_settings_view_rejects_invalid_utc_offset() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "dark".to_string(),
            share_url: None,
            utc_offset: "invalid".to_string(),
        };
        assert!(data.into_user_settings_view(user_id).is_err());
    }

    #[test]
    fn test_from_user_settings() -> ::anyhow::Result<()> {
        let settings = kernel::UserSettings::new(
            kernel::ColorScheme::Dark,
            Some("https://example.com/?u={{url}}".parse::<kernel::ShareUrl>()?),
            kernel::UserId::new(),
            kernel::UtcOffset::new(540)?,
        );
        let data = UserSettingsDocumentData::from_user_settings(&settings);
        assert_eq!(data.color_scheme, "dark");
        assert_eq!(
            data.share_url.as_deref(),
            Some("https://example.com/?u={{url}}")
        );
        assert_eq!(data.utc_offset, "+09:00");
        Ok(())
    }

    #[test]
    fn test_from_user_settings_without_share_url() {
        let settings = kernel::UserSettings::create(kernel::UserId::new());
        let data = UserSettingsDocumentData::from_user_settings(&settings);
        assert_eq!(data.share_url, None);
    }

    #[test]
    fn test_into_user_settings() -> ::anyhow::Result<()> {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "light".to_string(),
            share_url: Some("https://example.com/?u={{url}}".to_string()),
            utc_offset: "-05:00".to_string(),
        };
        let settings = data.into_user_settings(user_id)?;
        assert_eq!(settings.color_scheme(), kernel::ColorScheme::Light);
        assert_eq!(
            settings.share_url(),
            Some(&"https://example.com/?u={{url}}".parse::<kernel::ShareUrl>()?)
        );
        assert_eq!(settings.user_id(), user_id);
        assert_eq!(settings.utc_offset(), kernel::UtcOffset::new(-300)?);
        Ok(())
    }

    #[test]
    fn test_into_user_settings_rejects_invalid_share_url() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "light".to_string(),
            share_url: Some("not a url".to_string()),
            utc_offset: "-05:00".to_string(),
        };
        assert!(data.into_user_settings(user_id).is_err());
    }

    #[test]
    fn test_into_user_settings_rejects_invalid_utc_offset() {
        let user_id = kernel::UserId::new();
        let data = UserSettingsDocumentData {
            color_scheme: "light".to_string(),
            share_url: None,
            utc_offset: "invalid".to_string(),
        };
        assert!(data.into_user_settings(user_id).is_err());
    }
}
