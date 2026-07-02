use crate::StoredToken;
use crate::TokenExchange;
use crate::TokenStore;
use crate::build_authorization_request;
use crate::exchange_code;
use crate::receive_callback;

/// login フローの設定。OIDC エンドポイントと public client の資格情報、loopback ポートを持つ。
pub(crate) struct LoginConfig {
    pub auth_endpoint: String,
    pub client_id: String,
    pub client_secret: String,
    pub port: u16,
    pub token_endpoint: String,
}

impl LoginConfig {
    /// Google の認可 / トークンエンドポイントを用いた設定を作る。
    pub(crate) fn google(client_id: String, client_secret: String, port: u16) -> Self {
        Self {
            auth_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
            client_id,
            client_secret,
            port,
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
        }
    }

    /// loopback の redirect_uri (`http://127.0.0.1:<port>/callback`)。
    pub(crate) fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }
}

/// loopback + PKCE でログインし、取得した refresh_token を `TokenStore` へ保存する。
pub(crate) async fn run(config: LoginConfig) -> ::anyhow::Result<()> {
    let listener = ::tokio::net::TcpListener::bind(("127.0.0.1", config.port)).await?;
    let redirect_uri = config.redirect_uri();
    let authorization =
        build_authorization_request(&config.auth_endpoint, &config.client_id, &redirect_uri)?;

    // devcontainer 等ブラウザを自動起動できない環境も想定し、URL を表示する。
    eprintln!(
        "Open the following URL in your browser to authorize:\n\n{}\n",
        authorization.authorization_url
    );

    let callback = receive_callback(listener).await?;
    if callback.state != authorization.csrf_state {
        ::anyhow::bail!("CSRF state mismatch");
    }

    let token = exchange_code(
        &config.token_endpoint,
        &TokenExchange {
            client_id: &config.client_id,
            client_secret: &config.client_secret,
            code: &callback.code,
            pkce_verifier: &authorization.pkce_verifier,
            redirect_uri: &redirect_uri,
        },
    )
    .await?;

    let refresh_token = token
        .refresh_token
        .ok_or_else(|| ::anyhow::anyhow!("token endpoint did not return a refresh_token"))?;
    TokenStore::from_env()?.save(&StoredToken { refresh_token })?;

    eprintln!("Login complete. Token saved.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn google_sets_google_endpoints_and_keeps_credentials() {
        let config = LoginConfig::google("cid".to_string(), "secret".to_string(), 9787);
        assert_eq!(
            config.auth_endpoint,
            "https://accounts.google.com/o/oauth2/v2/auth"
        );
        assert_eq!(config.token_endpoint, "https://oauth2.googleapis.com/token");
        assert_eq!(config.client_id, "cid");
        assert_eq!(config.client_secret, "secret");
        assert_eq!(config.port, 9787);
    }

    #[test]
    fn redirect_uri_uses_loopback_and_port() {
        let config = LoginConfig::google("cid".to_string(), "secret".to_string(), 12345);
        assert_eq!(config.redirect_uri(), "http://127.0.0.1:12345/callback");
    }
}
