use crate::StoredToken;
use crate::TokenExchange;
use crate::TokenStore;
use crate::build_authorization_request;
use crate::exchange_code;
use crate::receive_callback;

// 公開リポジトリのため直書きせず、ビルド時に環境変数から焼き込む。
// デスクトップ型 client_secret は非機密でバイナリからは抽出可能だが、
// ソースにコミットしないことで secret スキャナによるクライアント自動無効化を避ける。
const EMBEDDED_CLIENT_ID: Option<&str> = option_env!("SHIORI_OIDC_CLIENT_ID");
const EMBEDDED_CLIENT_SECRET: Option<&str> = option_env!("SHIORI_OIDC_CLIENT_SECRET");

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

    /// ビルド時に焼き込まれた資格情報を用いて Google 設定を作る。
    /// 資格情報なしでビルドされていた場合はエラーにする。
    pub(crate) fn google_embedded(port: u16) -> ::anyhow::Result<Self> {
        let client_id = EMBEDDED_CLIENT_ID.ok_or_else(|| {
            ::anyhow::anyhow!("this binary was built without SHIORI_OIDC_CLIENT_ID")
        })?;
        let client_secret = EMBEDDED_CLIENT_SECRET.ok_or_else(|| {
            ::anyhow::anyhow!("this binary was built without SHIORI_OIDC_CLIENT_SECRET")
        })?;
        Ok(Self::google(
            client_id.to_string(),
            client_secret.to_string(),
            port,
        ))
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
    if let Err(e) = try_open_browser(&authorization.authorization_url) {
        eprintln!("Failed to open browser automatically: {e}");
    }

    let callback = receive_callback(listener).await?;
    if callback.state != authorization.state {
        ::anyhow::bail!("CSRF state mismatch");
    }

    let token = exchange_code(
        &config.token_endpoint,
        &TokenExchange {
            client_id: &config.client_id,
            client_secret: &config.client_secret,
            code: &callback.code,
            code_verifier: &authorization.code_verifier,
            grant_type: "authorization_code",
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

fn try_open_browser(url: &str) -> ::anyhow::Result<()> {
    let browser_env = ::std::env::var("BROWSER").ok();
    let (program, args) = browser_command(url, browser_env.as_deref());
    let status = ::std::process::Command::new(&program)
        .args(&args)
        .status()?;
    if status.success() {
        return Ok(());
    }
    ::anyhow::bail!("command exited with status {status}: {program}")
}

fn browser_command(url: &str, browser: Option<&str>) -> (String, Vec<String>) {
    if let Some(program) = browser.filter(|s| !s.is_empty()) {
        return (program.to_string(), vec![url.to_string()]);
    }

    #[cfg(target_os = "macos")]
    {
        return ("open".to_string(), vec![url.to_string()]);
    }
    #[cfg(target_os = "windows")]
    {
        return (
            "cmd".to_string(),
            vec!["/C".to_string(), "start".to_string(), url.to_string()],
        );
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        ("xdg-open".to_string(), vec![url.to_string()])
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

    #[test]
    fn browser_command_prefers_browser_env() {
        let (program, args) = browser_command("https://example.com", Some("firefox"));
        assert_eq!(program, "firefox");
        assert_eq!(args, vec!["https://example.com".to_string()]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn browser_command_uses_xdg_open_without_browser_env() {
        let (program, args) = browser_command("https://example.com", None);
        assert_eq!(program, "xdg-open");
        assert_eq!(args, vec!["https://example.com".to_string()]);
    }
}
