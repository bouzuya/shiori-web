// 0001-01-01T00:00:00.000Z
const MIN_MILLIS: i64 = -62_135_596_800_000;
// 9999-12-31T23:59:59.999Z
const MAX_MILLIS: i64 = 253_402_300_799_999;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DateTime {
    inner: chrono::DateTime<chrono::Utc>,
}

#[allow(dead_code)]
impl DateTime {
    fn from_millis(millis: i64) -> anyhow::Result<Self> {
        if !(MIN_MILLIS..=MAX_MILLIS).contains(&millis) {
            anyhow::bail!(
                "datetime out of range: must be between 0001-01-01T00:00:00.000Z and 9999-12-31T23:59:59.999Z"
            );
        }
        let secs = millis.div_euclid(1000);
        let nanos = (millis.rem_euclid(1000) * 1_000_000) as u32;
        let inner = chrono::TimeZone::timestamp_opt(&chrono::Utc, secs, nanos)
            .single()
            .ok_or_else(|| anyhow::anyhow!("timestamp out of range: {millis}"))?;
        Ok(Self { inner })
    }

    pub(crate) fn from_rfc3339(s: &str) -> anyhow::Result<Self> {
        // Validate millisecond precision: require exactly 3 fractional second digits.
        // RFC3339 format is YYYY-MM-DDTHH:MM:SS[.fraction]timezone, so the dot
        // appears at position 19 or later (after the fixed-length datetime prefix).
        let dot_pos = s.get(19..).and_then(|tail| tail.find('.')).map(|p| p + 19);
        match dot_pos {
            None => {
                anyhow::bail!("RFC3339 string must have millisecond precision (3 decimal places)")
            }
            Some(pos) => {
                let digits = s[pos + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .count();
                if digits != 3 {
                    anyhow::bail!(
                        "RFC3339 string must have millisecond precision (3 decimal places), got {digits}"
                    );
                }
            }
        }
        let dt = chrono::DateTime::parse_from_rfc3339(s)?;
        Self::from_millis(dt.timestamp_millis())
    }

    pub(crate) fn from_unix_timestamp(secs: i64) -> anyhow::Result<Self> {
        let millis = secs
            .checked_mul(1000)
            .ok_or_else(|| anyhow::anyhow!("unix timestamp overflow: {secs}"))?;
        Self::from_millis(millis)
    }

    pub(crate) fn now() -> Self {
        let millis = chrono::Utc::now().timestamp_millis();
        // System time is always within [0001-01-01, 9999-12-31], so from_millis cannot fail here.
        Self::from_millis(millis).unwrap_or_else(|_| unreachable!("system clock is out of range"))
    }

    pub(crate) fn from_unix_timestamp_as_millis(millis: i64) -> anyhow::Result<Self> {
        Self::from_millis(millis)
    }

    pub(crate) fn to_rfc3339(&self) -> String {
        self.inner
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    pub(crate) fn to_unix_timestamp(&self) -> i64 {
        self.inner.timestamp()
    }

    pub(crate) fn to_unix_timestamp_as_millis(&self) -> i64 {
        self.inner.timestamp_millis()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_rfc3339_and_to_rfc3339_roundtrip() -> anyhow::Result<()> {
        let s = "2024-01-15T10:30:45.123Z";
        let dt = DateTime::from_rfc3339(s)?;
        assert_eq!(dt.to_rfc3339(), s);
        Ok(())
    }

    #[test]
    fn test_from_rfc3339_invalid_returns_error() {
        assert!(DateTime::from_rfc3339("not-a-date").is_err());
    }

    #[test]
    fn test_from_rfc3339_without_fractional_seconds_returns_error() {
        assert!(DateTime::from_rfc3339("2024-01-15T10:30:45Z").is_err());
    }

    #[test]
    fn test_from_rfc3339_with_one_decimal_returns_error() {
        assert!(DateTime::from_rfc3339("2024-01-15T10:30:45.1Z").is_err());
    }

    #[test]
    fn test_from_rfc3339_with_two_decimals_returns_error() {
        assert!(DateTime::from_rfc3339("2024-01-15T10:30:45.12Z").is_err());
    }

    #[test]
    fn test_from_rfc3339_with_nanosecond_precision_returns_error() {
        assert!(DateTime::from_rfc3339("2024-01-15T10:30:45.123456789Z").is_err());
    }

    #[test]
    fn test_from_unix_timestamp_and_to_unix_timestamp_roundtrip() -> anyhow::Result<()> {
        let secs = 1705314645_i64;
        let dt = DateTime::from_unix_timestamp(secs)?;
        assert_eq!(dt.to_unix_timestamp(), secs);
        Ok(())
    }

    #[test]
    fn test_from_unix_timestamp_as_millis_and_to_millis_roundtrip() -> anyhow::Result<()> {
        let millis = 1705314645123_i64;
        let dt = DateTime::from_unix_timestamp_as_millis(millis)?;
        assert_eq!(dt.to_unix_timestamp_as_millis(), millis);
        Ok(())
    }

    #[test]
    fn test_from_unix_timestamp_to_rfc3339() -> anyhow::Result<()> {
        let dt = DateTime::from_unix_timestamp(0)?;
        assert_eq!(dt.to_rfc3339(), "1970-01-01T00:00:00.000Z");
        Ok(())
    }

    #[test]
    fn test_from_unix_timestamp_as_millis_to_rfc3339() -> anyhow::Result<()> {
        let dt = DateTime::from_unix_timestamp_as_millis(1705314645123)?;
        assert_eq!(dt.to_rfc3339(), "2024-01-15T10:30:45.123Z");
        Ok(())
    }

    #[test]
    fn test_to_unix_timestamp_truncates_millis() -> anyhow::Result<()> {
        let dt = DateTime::from_unix_timestamp_as_millis(1705314645999)?;
        assert_eq!(dt.to_unix_timestamp(), 1705314645);
        Ok(())
    }

    #[test]
    fn test_from_rfc3339_to_unix_timestamp_as_millis() -> anyhow::Result<()> {
        let dt = DateTime::from_rfc3339("2024-01-15T10:30:45.123Z")?;
        assert_eq!(dt.to_unix_timestamp_as_millis(), 1705314645123);
        Ok(())
    }

    #[test]
    fn test_min_value_is_accepted() -> anyhow::Result<()> {
        let dt = DateTime::from_rfc3339("0001-01-01T00:00:00.000Z")?;
        assert_eq!(dt.to_rfc3339(), "0001-01-01T00:00:00.000Z");
        assert_eq!(dt.to_unix_timestamp_as_millis(), MIN_MILLIS);
        Ok(())
    }

    #[test]
    fn test_max_value_is_accepted() -> anyhow::Result<()> {
        let dt = DateTime::from_rfc3339("9999-12-31T23:59:59.999Z")?;
        assert_eq!(dt.to_rfc3339(), "9999-12-31T23:59:59.999Z");
        assert_eq!(dt.to_unix_timestamp_as_millis(), MAX_MILLIS);
        Ok(())
    }

    #[test]
    fn test_before_min_returns_error() {
        assert!(DateTime::from_unix_timestamp_as_millis(MIN_MILLIS - 1).is_err());
    }

    #[test]
    fn test_after_max_returns_error() {
        assert!(DateTime::from_unix_timestamp_as_millis(MAX_MILLIS + 1).is_err());
    }

    #[test]
    fn test_now_returns_current_time() {
        let before_millis = chrono::Utc::now().timestamp_millis();
        let dt = DateTime::now();
        let after_millis = chrono::Utc::now().timestamp_millis();
        let millis = dt.to_unix_timestamp_as_millis();
        assert!(before_millis <= millis);
        assert!(millis <= after_millis);
    }

    #[test]
    fn test_clone() -> anyhow::Result<()> {
        let dt = DateTime::from_rfc3339("2024-01-15T10:30:45.123Z")?;
        let cloned = dt.clone();
        assert_eq!(dt, cloned);
        Ok(())
    }

    #[test]
    fn test_copy() -> anyhow::Result<()> {
        let dt = DateTime::from_rfc3339("2024-01-15T10:30:45.123Z")?;
        let copied = dt;
        // dt is still usable after copy
        assert_eq!(dt, copied);
        Ok(())
    }

    #[test]
    fn test_debug() -> anyhow::Result<()> {
        let dt = DateTime::from_rfc3339("2024-01-15T10:30:45.123Z")?;
        let s = format!("{dt:?}");
        assert!(!s.is_empty());
        Ok(())
    }

    #[test]
    fn test_eq() -> anyhow::Result<()> {
        let a = DateTime::from_rfc3339("2024-01-15T10:30:45.123Z")?;
        let b = DateTime::from_rfc3339("2024-01-15T10:30:45.123Z")?;
        let c = DateTime::from_rfc3339("2024-01-15T10:30:45.456Z")?;
        assert_eq!(a, b);
        assert_ne!(a, c);
        Ok(())
    }

    #[test]
    fn test_ord() -> anyhow::Result<()> {
        let earlier = DateTime::from_rfc3339("2024-01-15T10:30:45.000Z")?;
        let later = DateTime::from_rfc3339("2024-01-15T10:30:45.999Z")?;
        assert!(earlier < later);
        assert!(later > earlier);
        assert_eq!(earlier.cmp(&earlier), std::cmp::Ordering::Equal);
        Ok(())
    }

    #[test]
    fn test_partial_ord() -> anyhow::Result<()> {
        let earlier = DateTime::from_rfc3339("2024-01-15T10:30:45.000Z")?;
        let later = DateTime::from_rfc3339("2024-01-15T10:30:45.999Z")?;
        assert!(earlier.partial_cmp(&later) == Some(std::cmp::Ordering::Less));
        assert!(later.partial_cmp(&earlier) == Some(std::cmp::Ordering::Greater));
        assert!(earlier.partial_cmp(&earlier) == Some(std::cmp::Ordering::Equal));
        Ok(())
    }

    #[test]
    fn test_negative_unix_timestamp() -> anyhow::Result<()> {
        let dt = DateTime::from_unix_timestamp_as_millis(-1500)?;
        assert_eq!(dt.to_unix_timestamp_as_millis(), -1500);
        assert_eq!(dt.to_unix_timestamp(), -2);

        let dt = DateTime::from_unix_timestamp_as_millis(0)?;
        assert_eq!(dt.to_rfc3339(), "1970-01-01T00:00:00.000Z");
        assert_eq!(dt.to_unix_timestamp(), 0);
        let dt = DateTime::from_unix_timestamp_as_millis(-1)?;
        assert_eq!(dt.to_rfc3339(), "1969-12-31T23:59:59.999Z");
        assert_eq!(dt.to_unix_timestamp(), -1);
        let dt = DateTime::from_unix_timestamp_as_millis(-10)?;
        assert_eq!(dt.to_rfc3339(), "1969-12-31T23:59:59.990Z");
        assert_eq!(dt.to_unix_timestamp(), -1);
        let dt = DateTime::from_unix_timestamp_as_millis(-100)?;
        assert_eq!(dt.to_rfc3339(), "1969-12-31T23:59:59.900Z");
        assert_eq!(dt.to_unix_timestamp(), -1);
        let dt = DateTime::from_unix_timestamp_as_millis(-1000)?;
        assert_eq!(dt.to_rfc3339(), "1969-12-31T23:59:59.000Z");
        assert_eq!(dt.to_unix_timestamp(), -1);
        let dt = DateTime::from_unix_timestamp_as_millis(-1100)?;
        assert_eq!(dt.to_rfc3339(), "1969-12-31T23:59:58.900Z");
        assert_eq!(dt.to_unix_timestamp(), -2);
        Ok(())
    }
}
