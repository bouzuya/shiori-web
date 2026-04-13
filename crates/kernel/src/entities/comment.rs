/// ブックマークに付けるユーザーのコメント。空文字も可、最大 255 文字。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Comment(String);

impl std::fmt::Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Comment {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        anyhow::ensure!(
            s.chars().count() <= 255,
            "Comment must be at most 255 characters"
        );
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_from_str_empty() -> anyhow::Result<()> {
        let c = "".parse::<Comment>()?;
        assert_eq!(c.to_string(), "");
        Ok(())
    }

    #[test]
    fn test_comment_from_str_non_empty() -> anyhow::Result<()> {
        let c = "hello".parse::<Comment>()?;
        assert_eq!(c.to_string(), "hello");
        Ok(())
    }

    #[test]
    fn test_comment_display() -> anyhow::Result<()> {
        let c = "test comment".parse::<Comment>()?;
        assert_eq!(format!("{c}"), "test comment");
        Ok(())
    }

    #[test]
    fn test_comment_from_str_max_length() -> anyhow::Result<()> {
        let s = "a".repeat(255);
        let c = s.parse::<Comment>()?;
        assert_eq!(c.to_string().len(), 255);
        Ok(())
    }

    #[test]
    fn test_comment_from_str_too_long() {
        let s = "a".repeat(256);
        assert!(s.parse::<Comment>().is_err());
    }

    #[test]
    fn test_comment_eq() -> anyhow::Result<()> {
        let c1 = "abc".parse::<Comment>()?;
        let c2 = "abc".parse::<Comment>()?;
        let c3 = "xyz".parse::<Comment>()?;
        assert_eq!(c1, c2);
        assert_ne!(c1, c3);
        Ok(())
    }
}
