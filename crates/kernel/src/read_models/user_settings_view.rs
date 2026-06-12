#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserSettingsView {
    pub color_scheme: String,
    pub user_id: String,
}

#[cfg(test)]
impl UserSettingsView {
    pub fn for_test() -> Self {
        use crate::ColorScheme;
        use crate::UserId;

        Self {
            color_scheme: ColorScheme::for_test().to_string(),
            user_id: UserId::new().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorScheme;

    #[test]
    fn test_user_settings_view_fields() {
        let view = UserSettingsView {
            color_scheme: ColorScheme::Dark.to_string(),
            user_id: "01HX000000000000000000000U".to_string(),
        };
        assert_eq!(view.color_scheme, "dark");
        assert_eq!(view.user_id, "01HX000000000000000000000U");
    }

    #[test]
    fn test_user_settings_view_clone_eq() {
        let view = UserSettingsView::for_test();
        assert_eq!(view.clone(), view);
    }
}
