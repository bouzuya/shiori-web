use anyhow::Context as _;

/// アプリケーション全体で使用する環境変数。
pub(crate) struct Env {
    /// アプリケーションのベースパス (例: `/app`、デフォルト: `""`)
    pub base_path: String,
    /// クッキー署名用シークレット (64バイト以上必要)
    pub cookie_signing_secret: String,
    /// OIDC の client id
    pub oidc_client_id: String,
    /// OIDC の client secret
    pub oidc_client_secret: String,
    /// OIDC の Issuer URL (例: `https://accounts.google.com`)
    pub oidc_issuer_url: String,
    /// OIDC の認証コールバック URI (例: `http://localhost:3000/auth/callback`)
    pub oidc_redirect_uri: String,
    /// listen するポート番号 (デフォルト: `3000`)
    pub port: u16,
}

impl Env {
    pub fn from_env() -> anyhow::Result<Self> {
        fn read_var(name: &str) -> anyhow::Result<String> {
            std::env::var(name).with_context(|| format!("environment variable {name} is not set"))
        }

        Ok(Self {
            base_path: std::env::var("BASE_PATH").unwrap_or_default(),
            cookie_signing_secret: read_var("COOKIE_SIGNING_SECRET")?,
            oidc_client_id: read_var("OIDC_CLIENT_ID")?,
            oidc_client_secret: read_var("OIDC_CLIENT_SECRET")?,
            oidc_issuer_url: read_var("OIDC_ISSUER_URL")?,
            oidc_redirect_uri: read_var("OIDC_REDIRECT_URI")?,
            port: std::env::var("PORT")
                .ok()
                .map(|v| v.parse::<u16>())
                .transpose()
                .with_context(|| "environment variable PORT is not a valid port number")?
                .unwrap_or(3000),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_reads_all_variables() -> anyhow::Result<()> {
        temp_env::with_vars(
            [
                ("BASE_PATH", Some("/app")),
                (
                    "COOKIE_SIGNING_SECRET",
                    Some("test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding"),
                ),
                ("OIDC_CLIENT_ID", Some("test_client_id")),
                ("OIDC_CLIENT_SECRET", Some("test_client_secret")),
                ("OIDC_ISSUER_URL", Some("https://issuer.example.com")),
                (
                    "OIDC_REDIRECT_URI",
                    Some("http://localhost:3000/auth/callback"),
                ),
                ("PORT", Some("8080")),
            ],
            || {
                let env = Env::from_env()?;
                assert_eq!(env.base_path, "/app");
                assert_eq!(
                    env.cookie_signing_secret,
                    "test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding"
                );
                assert_eq!(env.oidc_client_id, "test_client_id");
                assert_eq!(env.oidc_client_secret, "test_client_secret");
                assert_eq!(env.oidc_issuer_url, "https://issuer.example.com");
                assert_eq!(env.oidc_redirect_uri, "http://localhost:3000/auth/callback");
                assert_eq!(env.port, 8080_u16);
                Ok(())
            },
        )
    }

    #[test]
    fn from_env_port_defaults_to_3000() -> anyhow::Result<()> {
        temp_env::with_vars(
            [
                ("BASE_PATH", None::<&str>),
                (
                    "COOKIE_SIGNING_SECRET",
                    Some("test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding"),
                ),
                ("OIDC_CLIENT_ID", Some("test_client_id")),
                ("OIDC_CLIENT_SECRET", Some("test_client_secret")),
                ("OIDC_ISSUER_URL", Some("https://issuer.example.com")),
                (
                    "OIDC_REDIRECT_URI",
                    Some("http://localhost:3000/auth/callback"),
                ),
                ("PORT", None::<&str>),
            ],
            || {
                let env = Env::from_env()?;
                assert_eq!(env.port, 3000_u16);
                Ok(())
            },
        )
    }

    #[test]
    fn from_env_base_path_defaults_to_empty() -> anyhow::Result<()> {
        temp_env::with_vars(
            [
                ("BASE_PATH", None::<&str>),
                (
                    "COOKIE_SIGNING_SECRET",
                    Some("test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding"),
                ),
                ("OIDC_CLIENT_ID", Some("test_client_id")),
                ("OIDC_CLIENT_SECRET", Some("test_client_secret")),
                ("OIDC_ISSUER_URL", Some("https://issuer.example.com")),
                (
                    "OIDC_REDIRECT_URI",
                    Some("http://localhost:3000/auth/callback"),
                ),
            ],
            || {
                let env = Env::from_env()?;
                assert_eq!(env.base_path, "");
                Ok(())
            },
        )
    }

    #[test]
    fn from_env_fails_when_variable_is_missing() -> anyhow::Result<()> {
        temp_env::with_vars(
            [
                ("BASE_PATH", None::<&str>),
                ("COOKIE_SIGNING_SECRET", None::<&str>),
                ("OIDC_CLIENT_ID", Some("test_client_id")),
                ("OIDC_CLIENT_SECRET", Some("secret")),
                ("OIDC_ISSUER_URL", Some("https://issuer.example.com")),
                (
                    "OIDC_REDIRECT_URI",
                    Some("http://localhost:3000/auth/callback"),
                ),
            ],
            || match Env::from_env() {
                Ok(_) => panic!("should fail"),
                Err(err) => assert!(
                    err.to_string().contains("COOKIE_SIGNING_SECRET"),
                    "error message should contain the variable name: {err}"
                ),
            },
        );
        Ok(())
    }
}
