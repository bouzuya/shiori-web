/// Google OIDC の sub claim を表す ID。
/// ASCII 文字のみ、最大 255 文字、大文字小文字を区別する。
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct UserId(String);

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for UserId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        anyhow::ensure!(s.is_ascii(), "UserId must be ASCII");
        anyhow::ensure!(s.len() <= 255, "UserId must be at most 255 characters");
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_from_str_valid() -> anyhow::Result<()> {
        let id = "abc123".parse::<UserId>()?;
        assert_eq!(id.to_string(), "abc123");
        Ok(())
    }

    #[test]
    fn test_user_id_from_str_case_sensitive() -> anyhow::Result<()> {
        let lower = "abc".parse::<UserId>()?;
        let upper = "ABC".parse::<UserId>()?;
        assert_ne!(lower, upper);
        Ok(())
    }

    #[test]
    fn test_user_id_from_str_max_length() -> anyhow::Result<()> {
        let s = "a".repeat(255);
        let id = s.parse::<UserId>()?;
        assert_eq!(id.to_string().len(), 255);
        Ok(())
    }

    #[test]
    fn test_user_id_from_str_too_long() {
        let s = "a".repeat(256);
        assert!(s.parse::<UserId>().is_err());
    }

    #[test]
    fn test_user_id_from_str_non_ascii() {
        assert!("あ".parse::<UserId>().is_err());
    }

    #[test]
    fn test_user_id_display() -> anyhow::Result<()> {
        let id = "abc123".parse::<UserId>()?;
        assert_eq!(format!("{id}"), "abc123");
        Ok(())
    }
}
