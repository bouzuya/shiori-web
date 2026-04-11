/// ユーザーを識別する ID。内部表現は UUIDv7。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UserId(uuid::Uuid);

#[allow(clippy::new_without_default)]
impl UserId {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for UserId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = uuid::Uuid::parse_str(s)?;
        Ok(Self(uuid))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_id_new_generates_unique_ids() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_user_id_new_is_v7() {
        let id = UserId::new();
        assert_eq!(id.0.get_version(), Some(uuid::Version::SortRand));
    }

    #[test]
    fn test_user_id_display_is_hyphenated_uuid() -> anyhow::Result<()> {
        let id = UserId::new();
        let s = id.to_string();
        let parsed = s.parse::<uuid::Uuid>()?;
        assert_eq!(parsed, id.0);
        Ok(())
    }

    #[test]
    fn test_user_id_from_str_roundtrip() -> anyhow::Result<()> {
        let id = UserId::new();
        let s = id.to_string();
        let parsed: UserId = s.parse()?;
        assert_eq!(parsed, id);
        Ok(())
    }

    #[test]
    fn test_user_id_from_str_invalid() {
        assert!("not-a-uuid".parse::<UserId>().is_err());
    }

    #[test]
    fn test_user_id_ordering_reflects_creation_order() {
        let id1 = UserId::new();
        let id2 = UserId::new();
        assert!(id1 < id2);
    }
}
