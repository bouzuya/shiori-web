use crate::ColorScheme;
use crate::UserId;
use crate::UtcOffset;

#[derive(Clone, Debug)]
pub struct UserSettings {
    color_scheme: ColorScheme,
    user_id: UserId,
    utc_offset: UtcOffset,
}

impl UserSettings {
    pub fn create(user_id: UserId) -> Self {
        Self {
            color_scheme: ColorScheme::default(),
            user_id,
            utc_offset: UtcOffset::default(),
        }
    }

    pub fn new(color_scheme: ColorScheme, user_id: UserId, utc_offset: UtcOffset) -> Self {
        Self {
            color_scheme,
            user_id,
            utc_offset,
        }
    }

    pub fn color_scheme(&self) -> ColorScheme {
        self.color_scheme
    }

    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    pub fn utc_offset(&self) -> UtcOffset {
        self.utc_offset
    }

    /// 配色設定を変更した新しい値を返す (識別子は維持する)。
    pub fn with_color_scheme(self, color_scheme: ColorScheme) -> Self {
        Self {
            color_scheme,
            ..self
        }
    }

    /// UTC オフセットを変更した新しい値を返す (識別子は維持する)。
    pub fn with_utc_offset(self, utc_offset: UtcOffset) -> Self {
        Self { utc_offset, ..self }
    }
}

#[cfg(test)]
impl UserSettings {
    pub fn for_test() -> Self {
        Self {
            color_scheme: ColorScheme::for_test(),
            user_id: UserId::new(),
            utc_offset: UtcOffset::for_test(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_defaults_to_system() {
        let user_id = UserId::new();
        let settings = UserSettings::create(user_id);
        assert_eq!(settings.color_scheme(), ColorScheme::System);
        assert_eq!(settings.user_id(), user_id);
        assert_eq!(settings.utc_offset(), UtcOffset::default());
    }

    #[test]
    fn test_new_stores_fields() -> anyhow::Result<()> {
        let user_id = UserId::new();
        let settings = UserSettings::new(ColorScheme::Dark, user_id, UtcOffset::new(540)?);
        assert_eq!(settings.color_scheme(), ColorScheme::Dark);
        assert_eq!(settings.user_id(), user_id);
        assert_eq!(settings.utc_offset(), UtcOffset::new(540)?);
        Ok(())
    }

    #[test]
    fn test_with_color_scheme_changes_scheme_and_keeps_user_id() {
        let settings = UserSettings::for_test();
        let user_id = settings.user_id();
        let changed = settings.with_color_scheme(ColorScheme::Light);
        assert_eq!(changed.color_scheme(), ColorScheme::Light);
        assert_eq!(changed.user_id(), user_id);
    }

    #[test]
    fn test_with_utc_offset_changes_offset_and_keeps_user_id() -> anyhow::Result<()> {
        let settings = UserSettings::for_test();
        let user_id = settings.user_id();
        let changed = settings.with_utc_offset(UtcOffset::new(-300)?);
        assert_eq!(changed.utc_offset(), UtcOffset::new(-300)?);
        assert_eq!(changed.user_id(), user_id);
        Ok(())
    }
}
