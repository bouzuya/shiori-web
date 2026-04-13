/// ブックマークのタイトル。空文字も可、最大 255 文字。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Title(String);

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Title {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        anyhow::ensure!(
            s.chars().count() <= 255,
            "Title must be at most 255 characters"
        );
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
impl Title {
    pub fn for_test() -> Self {
        use rand::RngExt as _;
        let mut rng = rand::rng();
        let len = rng.random_range(0..=255);
        let s: String = rng
            .sample_iter(rand::distr::Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();
        s.parse().expect("generated title must be valid")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_from_str_empty() -> anyhow::Result<()> {
        let t = "".parse::<Title>()?;
        assert_eq!(t.to_string(), "");
        Ok(())
    }

    #[test]
    fn test_title_from_str_non_empty() -> anyhow::Result<()> {
        let t = "My Article".parse::<Title>()?;
        assert_eq!(t.to_string(), "My Article");
        Ok(())
    }

    #[test]
    fn test_title_display() -> anyhow::Result<()> {
        let t = "test title".parse::<Title>()?;
        assert_eq!(format!("{t}"), "test title");
        Ok(())
    }

    #[test]
    fn test_title_from_str_max_length() -> anyhow::Result<()> {
        let s = "a".repeat(255);
        let t = s.parse::<Title>()?;
        assert_eq!(t.to_string().len(), 255);
        Ok(())
    }

    #[test]
    fn test_title_from_str_too_long() {
        let s = "a".repeat(256);
        assert!(s.parse::<Title>().is_err());
    }

    #[test]
    fn test_title_eq() -> anyhow::Result<()> {
        let t1 = "abc".parse::<Title>()?;
        let t2 = "abc".parse::<Title>()?;
        let t3 = "xyz".parse::<Title>()?;
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
        Ok(())
    }
}
