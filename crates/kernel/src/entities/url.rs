/// ブックマークの URL。http または https スキームのみ許可。
/// 非 ASCII は percent-encode / punycode 変換後に最大 2048 文字。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Url(url::Url);

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Url {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parsed = url::Url::parse(s)?;
        anyhow::ensure!(
            matches!(parsed.scheme(), "http" | "https"),
            "URL scheme must be http or https"
        );
        anyhow::ensure!(
            parsed.as_str().len() <= 2048,
            "URL must be at most 2048 characters"
        );
        Ok(Self(parsed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_from_str_http() -> anyhow::Result<()> {
        let u = "http://example.com".parse::<Url>()?;
        assert_eq!(u.to_string(), "http://example.com/");
        Ok(())
    }

    #[test]
    fn test_url_from_str_https() -> anyhow::Result<()> {
        let u = "https://example.com/path?q=1".parse::<Url>()?;
        assert_eq!(u.to_string(), "https://example.com/path?q=1");
        Ok(())
    }

    #[test]
    fn test_url_from_str_non_ascii_host_is_punycoded() -> anyhow::Result<()> {
        // 非 ASCII ホストは punycode に変換される
        let u = "https://例え.jp".parse::<Url>()?;
        assert_eq!(u.to_string(), "https://xn--r8jz45g.jp/");
        Ok(())
    }

    #[test]
    fn test_url_from_str_non_ascii_path_is_percent_encoded() -> anyhow::Result<()> {
        // 非 ASCII パスは percent-encode される
        let u = "https://example.com/日本語".parse::<Url>()?;
        assert_eq!(
            u.to_string(),
            "https://example.com/%E6%97%A5%E6%9C%AC%E8%AA%9E"
        );
        Ok(())
    }

    #[test]
    fn test_url_from_str_invalid() {
        assert!("not a url".parse::<Url>().is_err());
    }

    #[test]
    fn test_url_from_str_non_http_scheme() {
        assert!("ftp://example.com".parse::<Url>().is_err());
    }

    #[test]
    fn test_url_display() -> anyhow::Result<()> {
        let u = "https://example.com".parse::<Url>()?;
        assert_eq!(format!("{u}"), "https://example.com/");
        Ok(())
    }

    #[test]
    fn test_url_from_str_max_length() -> anyhow::Result<()> {
        let path = "a".repeat(2048 - "https://e.com/".len());
        let s = format!("https://e.com/{path}");
        assert_eq!(s.len(), 2048);
        let u = s.parse::<Url>()?;
        assert_eq!(u.to_string().len(), 2048);
        Ok(())
    }

    #[test]
    fn test_url_from_str_too_long() {
        let path = "a".repeat(2048 - "https://e.com/".len() + 1);
        let s = format!("https://e.com/{path}");
        assert_eq!(s.len(), 2049);
        assert!(s.parse::<Url>().is_err());
    }

    #[test]
    fn test_url_eq() -> anyhow::Result<()> {
        let u1 = "https://example.com".parse::<Url>()?;
        let u2 = "https://example.com".parse::<Url>()?;
        let u3 = "https://other.com".parse::<Url>()?;
        assert_eq!(u1, u2);
        assert_ne!(u1, u3);
        Ok(())
    }
}
