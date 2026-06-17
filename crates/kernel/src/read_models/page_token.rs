/// 一覧ページネーションのカーソルを表す不透明トークン。
///
/// `Display` で不透明な hex 文字列へ、`FromStr` で復元する。生の `created_at` は
/// 露出しない。トークン無し (最新ページ) は `Option<PageToken>` の `None` で表す。
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PageToken {
    /// `created_at` より古い側 (次ページ)。
    Next(String),
    /// `created_at` より新しい側 (前ページ)。
    Prev(String),
}

impl std::fmt::Display for PageToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (prefix, created_at) = match self {
            Self::Next(created_at) => (b'n', created_at),
            Self::Prev(created_at) => (b'p', created_at),
        };
        write!(f, "{prefix:02x}")?;
        for byte in created_at.as_bytes() {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for PageToken {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn hex_value(byte: u8) -> anyhow::Result<u8> {
            match byte {
                b'0'..=b'9' => Ok(byte - b'0'),
                b'a'..=b'f' => Ok(byte - b'a' + 10),
                b'A'..=b'F' => Ok(byte - b'A' + 10),
                _ => anyhow::bail!("invalid hex digit"),
            }
        }

        let bytes = s.as_bytes();
        anyhow::ensure!(
            bytes.len().is_multiple_of(2),
            "page token must have even length"
        );
        let mut decoded = Vec::with_capacity(bytes.len() / 2);
        for pair in bytes.chunks_exact(2) {
            decoded.push(hex_value(pair[0])? << 4 | hex_value(pair[1])?);
        }
        let payload = String::from_utf8(decoded)?;
        let mut chars = payload.chars();
        let direction = chars
            .next()
            .ok_or_else(|| anyhow::anyhow!("page token is empty"))?;
        let created_at = chars.as_str().to_string();
        match direction {
            'n' => Ok(Self::Next(created_at)),
            'p' => Ok(Self::Prev(created_at)),
            _ => anyhow::bail!("invalid page token direction"),
        }
    }
}

#[cfg(test)]
impl PageToken {
    pub fn for_test() -> Self {
        use rand::RngExt as _;
        let mut rng = rand::rng();
        let after = rng.random_range(0..2) == 0;
        let len = rng.random_range(1..=20);
        let created_at: String = rng
            .sample_iter(rand::distr::Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();
        if after {
            Self::Next(created_at)
        } else {
            Self::Prev(created_at)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_roundtrip() -> anyhow::Result<()> {
        let token = PageToken::Next("2024-01-15T20:00:00.000Z".to_string());
        let encoded = token.to_string();
        assert_eq!(encoded.parse::<PageToken>()?, token);
        Ok(())
    }

    #[test]
    fn test_prev_roundtrip() -> anyhow::Result<()> {
        let token = PageToken::Prev("2024-01-15T20:00:00.000Z".to_string());
        let encoded = token.to_string();
        assert_eq!(encoded.parse::<PageToken>()?, token);
        Ok(())
    }

    #[test]
    fn test_for_test_roundtrip() -> anyhow::Result<()> {
        let token = PageToken::for_test();
        let encoded = token.to_string();
        assert_eq!(encoded.parse::<PageToken>()?, token);
        Ok(())
    }

    #[test]
    fn test_display_is_opaque() -> anyhow::Result<()> {
        let token = PageToken::Next("2024-01-15T20:00:00.000Z".to_string());
        let encoded = token.to_string();
        // 生の created_at (コロンを含む RFC3339) が露出しないこと
        assert_ne!(encoded, "2024-01-15T20:00:00.000Z");
        assert!(!encoded.contains(':'));
        Ok(())
    }

    #[test]
    fn test_from_str_empty_is_err() {
        assert!("".parse::<PageToken>().is_err());
    }

    #[test]
    fn test_from_str_invalid_hex_is_err() {
        assert!("zz".parse::<PageToken>().is_err());
    }

    #[test]
    fn test_from_str_odd_length_is_err() {
        assert!("616".parse::<PageToken>().is_err());
    }

    #[test]
    fn test_from_str_unknown_direction_is_err() -> anyhow::Result<()> {
        let valid = PageToken::Next("foo".to_string()).to_string();
        // 先頭の方向接頭辞 'n' (0x6e -> "6e") を 'x' (0x78 -> "78") に差し替える
        let bad = format!("78{}", &valid[2..]);
        assert!(bad.parse::<PageToken>().is_err());
        Ok(())
    }
}
