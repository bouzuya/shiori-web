/// アプリケーション全体で使用する環境変数。
pub(crate) struct Env {
    /// OIDC の client id
    pub oidc_client_id: String,
    /// OIDC の client secret
    pub oidc_client_secret: String,
    /// OIDC の Issuer URL (例: `https://accounts.google.com`)
    pub oidc_issuer_url: String,
    /// OIDC の認証コールバック URI (例: `http://localhost:3000/auth/callback`)
    pub oidc_redirect_uri: String,
}

impl Env {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            oidc_client_id: std::env::var("OIDC_CLIENT_ID")?,
            oidc_client_secret: std::env::var("OIDC_CLIENT_SECRET")?,
            oidc_issuer_url: std::env::var("OIDC_ISSUER_URL")?,
            oidc_redirect_uri: std::env::var("OIDC_REDIRECT_URI")?,
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
                assert_eq!(env.oidc_client_id, "test_client_id");
                assert_eq!(env.oidc_client_secret, "test_client_secret");
                assert_eq!(env.oidc_issuer_url, "https://issuer.example.com");
                assert_eq!(env.oidc_redirect_uri, "http://localhost:3000/auth/callback");
                Ok(())
            },
        )
    }

    #[test]
    fn from_env_fails_when_variable_is_missing() {
        temp_env::with_var_unset("OIDC_CLIENT_ID", || {
            assert!(Env::from_env().is_err());
        });
    }
}
