/// ユーザーの配色設定。`data-color-scheme` 属性値とそのまま対応する。
/// `System` は OS 設定 (`prefers-color-scheme`) に追従することを表す。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ColorScheme {
    Dark,
    Light,
    #[default]
    System,
}

impl ::std::fmt::Display for ColorScheme {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        let s = match self {
            ColorScheme::Dark => "dark",
            ColorScheme::Light => "light",
            ColorScheme::System => "system",
        };
        s.fmt(f)
    }
}

impl ::std::str::FromStr for ColorScheme {
    type Err = ::anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dark" => Ok(ColorScheme::Dark),
            "light" => Ok(ColorScheme::Light),
            "system" => Ok(ColorScheme::System),
            _ => ::anyhow::bail!("invalid ColorScheme: {s}"),
        }
    }
}

#[cfg(test)]
impl ColorScheme {
    pub fn for_test() -> Self {
        let mut rng = ::rand::rng();
        *::rand::seq::IndexedRandom::sample(
            [ColorScheme::Dark, ColorScheme::Light, ColorScheme::System].as_slice(),
            &mut rng,
            1,
        )
        .next()
        .expect("non-empty")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_scheme_display() {
        assert_eq!(ColorScheme::Dark.to_string(), "dark");
        assert_eq!(ColorScheme::Light.to_string(), "light");
        assert_eq!(ColorScheme::System.to_string(), "system");
    }

    #[test]
    fn test_color_scheme_from_str() -> ::anyhow::Result<()> {
        assert_eq!("dark".parse::<ColorScheme>()?, ColorScheme::Dark);
        assert_eq!("light".parse::<ColorScheme>()?, ColorScheme::Light);
        assert_eq!("system".parse::<ColorScheme>()?, ColorScheme::System);
        Ok(())
    }

    #[test]
    fn test_color_scheme_display_then_from_str_roundtrip() -> ::anyhow::Result<()> {
        for variant in [ColorScheme::Dark, ColorScheme::Light, ColorScheme::System] {
            assert_eq!(variant.to_string().parse::<ColorScheme>()?, variant);
        }
        Ok(())
    }

    #[test]
    fn test_color_scheme_from_str_invalid() {
        assert!("".parse::<ColorScheme>().is_err());
        assert!("Dark".parse::<ColorScheme>().is_err());
        assert!("auto".parse::<ColorScheme>().is_err());
    }

    #[test]
    fn test_color_scheme_default_is_system() {
        assert_eq!(ColorScheme::default(), ColorScheme::System);
    }
}
