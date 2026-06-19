/// UTC からの固定オフセット。内部は分で保持し、`"+09:00"` 形式で表示・解析する。
/// DST は扱わない。既定値は UTC (`+00:00`)。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UtcOffset {
    minutes: i32,
}

impl UtcOffset {
    // -12:00 〜 +14:00
    const MAX_MINUTES: i32 = 840;
    const MIN_MINUTES: i32 = -720;

    pub fn new(minutes: i32) -> anyhow::Result<Self> {
        if !(Self::MIN_MINUTES..=Self::MAX_MINUTES).contains(&minutes) {
            anyhow::bail!("UtcOffset out of range: {minutes}");
        }
        Ok(Self { minutes })
    }

    pub fn minutes(&self) -> i32 {
        self.minutes
    }
}

impl std::fmt::Display for UtcOffset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.minutes < 0 { '-' } else { '+' };
        let abs = self.minutes.abs();
        write!(f, "{sign}{:02}:{:02}", abs / 60, abs % 60)
    }
}

impl std::str::FromStr for UtcOffset {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let sign = match s.as_bytes().first() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => anyhow::bail!("invalid UtcOffset: {s}"),
        };
        let (hours, mins) = s[1..]
            .split_once(':')
            .ok_or_else(|| anyhow::anyhow!("invalid UtcOffset: {s}"))?;
        if hours.len() != 2
            || mins.len() != 2
            || !hours.bytes().all(|b| b.is_ascii_digit())
            || !mins.bytes().all(|b| b.is_ascii_digit())
        {
            anyhow::bail!("invalid UtcOffset: {s}");
        }
        let hours = hours.parse::<i32>()?;
        let mins = mins.parse::<i32>()?;
        if mins >= 60 {
            anyhow::bail!("invalid UtcOffset: {s}");
        }
        Self::new(sign * (hours * 60 + mins))
    }
}

#[cfg(test)]
impl UtcOffset {
    pub fn for_test() -> Self {
        let mut rng = rand::rng();
        rand::seq::IndexedRandom::sample(
            [
                UtcOffset { minutes: -720 },
                UtcOffset { minutes: -300 },
                UtcOffset { minutes: 0 },
                UtcOffset { minutes: 330 },
                UtcOffset { minutes: 540 },
                UtcOffset { minutes: 840 },
            ]
            .as_slice(),
            &mut rng,
            1,
        )
        .next()
        .copied()
        .expect("non-empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utc_offset_display() -> anyhow::Result<()> {
        assert_eq!(UtcOffset::new(540)?.to_string(), "+09:00");
        assert_eq!(UtcOffset::new(-300)?.to_string(), "-05:00");
        assert_eq!(UtcOffset::new(0)?.to_string(), "+00:00");
        assert_eq!(UtcOffset::new(330)?.to_string(), "+05:30");
        Ok(())
    }

    #[test]
    fn test_utc_offset_from_str() -> anyhow::Result<()> {
        assert_eq!("+09:00".parse::<UtcOffset>()?, UtcOffset::new(540)?);
        assert_eq!("-05:00".parse::<UtcOffset>()?, UtcOffset::new(-300)?);
        assert_eq!("+00:00".parse::<UtcOffset>()?, UtcOffset::new(0)?);
        assert_eq!("+05:30".parse::<UtcOffset>()?, UtcOffset::new(330)?);
        Ok(())
    }

    #[test]
    fn test_utc_offset_display_then_from_str_roundtrip() -> anyhow::Result<()> {
        for minutes in [-720, -300, 0, 60, 330, 540, 840] {
            let offset = UtcOffset::new(minutes)?;
            assert_eq!(offset.to_string().parse::<UtcOffset>()?, offset);
        }
        Ok(())
    }

    #[test]
    fn test_utc_offset_new_out_of_range() {
        assert!(UtcOffset::new(841).is_err());
        assert!(UtcOffset::new(-721).is_err());
    }

    #[test]
    fn test_utc_offset_from_str_out_of_range() {
        assert!("+15:00".parse::<UtcOffset>().is_err());
        assert!("-13:00".parse::<UtcOffset>().is_err());
    }

    #[test]
    fn test_utc_offset_from_str_invalid_format() {
        assert!("".parse::<UtcOffset>().is_err());
        assert!("09:00".parse::<UtcOffset>().is_err());
        assert!("+9:00".parse::<UtcOffset>().is_err());
        assert!("+09:0".parse::<UtcOffset>().is_err());
        assert!("+09:99".parse::<UtcOffset>().is_err());
        assert!("+0a:00".parse::<UtcOffset>().is_err());
        assert!("abc".parse::<UtcOffset>().is_err());
    }

    #[test]
    fn test_utc_offset_minutes() -> anyhow::Result<()> {
        assert_eq!(UtcOffset::new(540)?.minutes(), 540);
        Ok(())
    }

    #[test]
    fn test_utc_offset_default_is_utc() {
        assert_eq!(UtcOffset::default().minutes(), 0);
    }
}
