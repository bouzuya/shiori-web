/// Google OIDC の sub claim を表す ID。
/// ASCII 文字のみ、最大 255 文字、大文字小文字を区別する。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GoogleUserId(String);

impl std::fmt::Display for GoogleUserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for GoogleUserId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        anyhow::ensure!(s.is_ascii(), "GoogleUserId must be ASCII");
        anyhow::ensure!(
            s.len() <= 255,
            "GoogleUserId must be at most 255 characters"
        );
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_user_id_from_str_valid() -> anyhow::Result<()> {
        let id = "abc123".parse::<GoogleUserId>()?;
        assert_eq!(id.to_string(), "abc123");
        Ok(())
    }

    #[test]
    fn test_google_user_id_from_str_case_sensitive() -> anyhow::Result<()> {
        let lower = "abc".parse::<GoogleUserId>()?;
        let upper = "ABC".parse::<GoogleUserId>()?;
        assert_ne!(lower, upper);
        Ok(())
    }

    #[test]
    fn test_google_user_id_from_str_max_length() -> anyhow::Result<()> {
        let s = "a".repeat(255);
        let id = s.parse::<GoogleUserId>()?;
        assert_eq!(id.to_string().len(), 255);
        Ok(())
    }

    #[test]
    fn test_google_user_id_from_str_too_long() {
        let s = "a".repeat(256);
        assert!(s.parse::<GoogleUserId>().is_err());
    }

    #[test]
    fn test_google_user_id_from_str_non_ascii() {
        assert!("あ".parse::<GoogleUserId>().is_err());
    }

    #[test]
    fn test_google_user_id_display() -> anyhow::Result<()> {
        let id = "abc123".parse::<GoogleUserId>()?;
        assert_eq!(format!("{id}"), "abc123");
        Ok(())
    }
}
