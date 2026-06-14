#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserSettingsView {
    pub color_scheme: String,
    pub share_url: Option<String>,
    pub user_id: String,
    pub utc_offset: String,
}

#[cfg(test)]
impl UserSettingsView {
    pub fn for_test() -> Self {
        use crate::ColorScheme;
        use crate::ShareUrl;
        use crate::UserId;
        use crate::UtcOffset;

        Self {
            color_scheme: ColorScheme::for_test().to_string(),
            share_url: Some(ShareUrl::for_test().to_string()),
            user_id: UserId::new().to_string(),
            utc_offset: UtcOffset::for_test().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorScheme;
    use crate::UtcOffset;

    #[test]
    fn test_user_settings_view_fields() {
        let view = UserSettingsView {
            color_scheme: ColorScheme::Dark.to_string(),
            share_url: Some("https://example.com/?u={{url}}".to_string()),
            user_id: "01HX000000000000000000000U".to_string(),
            utc_offset: UtcOffset::default().to_string(),
        };
        assert_eq!(view.color_scheme, "dark");
        assert_eq!(
            view.share_url.as_deref(),
            Some("https://example.com/?u={{url}}")
        );
        assert_eq!(view.user_id, "01HX000000000000000000000U");
        assert_eq!(view.utc_offset, "+00:00");
    }

    #[test]
    fn test_user_settings_view_clone_eq() {
        let view = UserSettingsView::for_test();
        assert_eq!(view.clone(), view);
    }
}
