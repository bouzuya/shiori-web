// login の orchestration (次の単位) が消費するまで bin ビルドでは未使用。消費側を追加したら外す。
#![allow(dead_code)]

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
