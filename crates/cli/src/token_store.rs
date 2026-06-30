// login / export が消費するまで bin ビルドでは未使用。消費側 (次の単位) を追加したら外す。
#![allow(dead_code)]

const APP_DIR: &str = "shiori";
const TOKEN_FILE: &str = "token.json";

#[derive(Clone, Debug, Eq, PartialEq, ::serde::Deserialize, ::serde::Serialize)]
pub(crate) struct StoredToken {
    pub refresh_token: String,
}

#[cfg(test)]
impl StoredToken {
    pub(crate) fn for_test() -> Self {
        let nanos = ::std::time::SystemTime::now()
            .duration_since(::std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self {
            refresh_token: format!("refresh-token-{nanos}"),
        }
    }
}

/// `$XDG_STATE_HOME/shiori/token.json` (未設定なら `$HOME/.local/state/shiori/token.json`)
/// に保存された OIDC トークンを読み書きする。
pub(crate) struct TokenStore {
    path: ::std::path::PathBuf,
}

impl TokenStore {
    pub(crate) fn new(state_dir: impl AsRef<::std::path::Path>) -> Self {
        Self {
            path: state_dir.as_ref().join(APP_DIR).join(TOKEN_FILE),
        }
    }

    pub(crate) fn from_env() -> ::anyhow::Result<Self> {
        let state_dir = resolve_state_dir(
            ::std::env::var("XDG_STATE_HOME").ok().as_deref(),
            ::std::env::var("HOME").ok().as_deref(),
        )?;
        Ok(Self::new(state_dir))
    }

    pub(crate) fn load(&self) -> ::anyhow::Result<Option<StoredToken>> {
        match ::std::fs::read_to_string(&self.path) {
            Ok(contents) => Ok(Some(::serde_json::from_str(&contents)?)),
            Err(e) if e.kind() == ::std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(::anyhow::anyhow!(e)),
        }
    }

    pub(crate) fn save(&self, token: &StoredToken) -> ::anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            ::std::fs::create_dir_all(parent)?;
        }
        ::std::fs::write(&self.path, ::serde_json::to_string(token)?)?;
        set_permissions_0600(&self.path)?;
        Ok(())
    }
}

fn resolve_state_dir(
    xdg_state_home: Option<&str>,
    home: Option<&str>,
) -> ::anyhow::Result<::std::path::PathBuf> {
    if let Some(dir) = xdg_state_home.filter(|s| !s.is_empty()) {
        return Ok(::std::path::PathBuf::from(dir));
    }
    let home = home
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ::anyhow::anyhow!("neither XDG_STATE_HOME nor HOME is set"))?;
    Ok(::std::path::PathBuf::from(home)
        .join(".local")
        .join("state"))
}

#[cfg(unix)]
fn set_permissions_0600(path: &::std::path::Path) -> ::anyhow::Result<()> {
    let mut permissions = ::std::fs::metadata(path)?.permissions();
    ::std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o600);
    ::std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions_0600(_path: &::std::path::Path) -> ::anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_then_load_round_trips() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let store = TokenStore::new(dir.path());
        let token = StoredToken::for_test();
        store.save(&token)?;
        assert_eq!(store.load()?, Some(token));
        Ok(())
    }

    #[test]
    fn load_returns_none_when_file_is_missing() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let store = TokenStore::new(dir.path());
        assert_eq!(store.load()?, None);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn saved_file_has_0600_permissions() -> ::anyhow::Result<()> {
        let dir = ::tempfile::tempdir()?;
        let store = TokenStore::new(dir.path());
        store.save(&StoredToken::for_test())?;
        let permissions = ::std::fs::metadata(&store.path)?.permissions();
        let mode = ::std::os::unix::fs::PermissionsExt::mode(&permissions);
        assert_eq!(mode & 0o777, 0o600);
        Ok(())
    }

    #[test]
    fn resolve_state_dir_prefers_xdg_state_home() -> ::anyhow::Result<()> {
        let dir = resolve_state_dir(Some("/xdg/state"), Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/xdg/state"));
        Ok(())
    }

    #[test]
    fn resolve_state_dir_falls_back_to_home_local_state() -> ::anyhow::Result<()> {
        let dir = resolve_state_dir(None, Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/home/u/.local/state"));
        Ok(())
    }

    #[test]
    fn resolve_state_dir_treats_empty_xdg_as_unset() -> ::anyhow::Result<()> {
        let dir = resolve_state_dir(Some(""), Some("/home/u"))?;
        assert_eq!(dir, ::std::path::PathBuf::from("/home/u/.local/state"));
        Ok(())
    }

    #[test]
    fn resolve_state_dir_errors_without_xdg_or_home() {
        assert!(resolve_state_dir(None, None).is_err());
    }
}
