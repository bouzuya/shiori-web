/// ブックマークを識別する ID。内部表現は UUIDv7。
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BookmarkId(uuid::Uuid);

#[allow(clippy::new_without_default)]
impl BookmarkId {
    pub fn new() -> Self {
        Self(uuid::Uuid::now_v7())
    }
}

impl std::fmt::Display for BookmarkId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for BookmarkId {
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
    fn test_bookmark_id_new_generates_unique_ids() {
        let id1 = BookmarkId::new();
        let id2 = BookmarkId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_bookmark_id_new_is_v7() {
        let id = BookmarkId::new();
        assert_eq!(id.0.get_version(), Some(uuid::Version::SortRand));
    }

    #[test]
    fn test_bookmark_id_display_is_hyphenated_uuid() -> anyhow::Result<()> {
        let id = BookmarkId::new();
        let s = id.to_string();
        let parsed = s.parse::<uuid::Uuid>()?;
        assert_eq!(parsed, id.0);
        Ok(())
    }

    #[test]
    fn test_bookmark_id_from_str_roundtrip() -> anyhow::Result<()> {
        let id = BookmarkId::new();
        let s = id.to_string();
        let parsed: BookmarkId = s.parse()?;
        assert_eq!(parsed, id);
        Ok(())
    }

    #[test]
    fn test_bookmark_id_from_str_invalid() {
        assert!("not-a-uuid".parse::<BookmarkId>().is_err());
    }

    #[test]
    fn test_bookmark_id_ordering_reflects_creation_order() {
        let id1 = BookmarkId::new();
        let id2 = BookmarkId::new();
        assert!(id1 < id2);
    }
}
