const APP_DIR: &str = "shiori";
const CONFIG_FILE: &str = "config.json";

#[derive(Clone, Debug, Eq, PartialEq, ::serde::Deserialize, ::serde::Serialize)]
pub(crate) struct StoredConfig {
    pub server_url: String,
}

#[cfg(test)]
impl StoredConfig {
    pub(crate) fn for_test() -> Self {
        let nanos = ::std::time::SystemTime::now()
            .duration_since(::std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self {
            server_url: format!("https://server-{nanos}.example.com"),
        }
    }
}

/// `$XDG_CONFIG_HOME/shiori/config.json` (未設定なら `$HOME/.config/shiori/config.json`)
/// に保存された CLI 設定を読み書きする。
pub(crate) struct ConfigStore {
    path: ::std::path::PathBuf,
}

impl ConfigStore {
    pub(crate) fn new(config_dir: impl AsRef<::std::path::Path>) -> Self {
        Self {
            path: config_dir.as_ref().join(APP_DIR).join(CONFIG_FILE),
        }
    }

    pub(crate) fn from_env() -> ::anyhow::Result<Self> {
        let config_dir = resolve_config_dir(
            ::std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
            ::std::env::var("HOME").ok().as_deref(),
        )?;
        Ok(Self::new(config_dir))
    }

    // Step 5 の export が消費するまで bin では未使用。
    #[allow(dead_code)]
    pub(crate) fn load(&self) -> ::anyhow::Result<Option<StoredConfig>> {
        match ::std::fs::read_to_string(&self.path) {
            Ok(contents) => Ok(Some(::serde_json::from_str(&contents)?)),
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(::anyhow::anyhow!(e)),
        }
    }

    pub(crate) fn save(&self, config: &StoredConfig) -> ::anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            ::std::fs::create_dir_all(parent)?;
        }
        ::std::fs::write(&self.path, ::serde_json::to_string(config)?)?;
        Ok(())
    }
}

fn resolve_config_dir(
    xdg_config_home: Option<&str>,
    home: Option<&str>,
) -> ::anyhow::Result<::std::path::PathBuf> {
    if let Some(dir) = xdg_config_home.filter(|s| !s.is_empty()) {
        return Ok(::std::path::PathBuf::from(dir));
    }
    let home = home
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ::anyhow::anyhow!("neither XDG_CONFIG_HOME nor HOME is set"))?;
    Ok(::std::path::PathBuf::from(home).join(".config"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_round_trips() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let store = ConfigStore::new(dir.path());
        let config = StoredConfig::for_test();
        store.save(&config)?;
        assert_eq!(store.load()?, Some(config));
        Ok(())
    }

    #[test]
    fn load_returns_none_when_file_is_missing() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let store = ConfigStore::new(dir.path());
        assert_eq!(store.load()?, None);
        Ok(())
    }

    #[test]
    fn saves_under_shiori_config_json() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let store = ConfigStore::new(dir.path());
        store.save(&StoredConfig::for_test())?;
        assert!(dir.path().join("shiori").join("config.json").is_file());
        Ok(())
    }

    #[test]
    fn resolve_config_dir_prefers_xdg_config_home() -> ::anyhow::Result<()> {
        let dir = resolve_config_dir(Some("/xdg/config"), Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/xdg/config"));
        Ok(())
    }

    #[test]
    fn resolve_config_dir_falls_back_to_home_dot_config() -> ::anyhow::Result<()> {
        let dir = resolve_config_dir(None, Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/home/u/.config"));
        Ok(())
    }

    #[test]
    fn resolve_config_dir_treats_empty_xdg_as_unset() -> ::anyhow::Result<()> {
        let dir = resolve_config_dir(Some(""), Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/home/u/.config"));
        Ok(())
    }

    #[test]
    fn resolve_config_dir_errors_without_xdg_or_home() {
        assert!(resolve_config_dir(None, None).is_err());
    }
}
