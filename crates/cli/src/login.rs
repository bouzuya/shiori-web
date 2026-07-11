use crate::ConfigStore;
use crate::StoredConfig;
use crate::StoredToken;
use crate::TokenExchange;
use crate::TokenStore;
use crate::build_authorization_request;
use crate::exchange_code;
use crate::fetch_provider_metadata;
use crate::fetch_server_config;
use crate::receive_callback;

/// login フローの設定。OIDC エンドポイントと public client の資格情報、loopback ポートを持つ。
pub(crate) struct LoginConfig {
    pub auth_endpoint: String,
    pub client_id: String,
    pub client_secret: String,
    pub port: u16,
    pub server_url: String,
    pub token_endpoint: String,
}

impl LoginConfig {
    /// サーバーの `/cli/config` と issuer の OIDC Discovery から設定を組み立てる。
    pub(crate) async fn fetch(server_url: &str, port: u16) -> ::anyhow::Result<Self> {
        let server_config = fetch_server_config(server_url).await?;
        let metadata = fetch_provider_metadata(&server_config.issuer).await?;
        Ok(Self {
            auth_endpoint: metadata.authorization_endpoint,
            client_id: server_config.client_id,
            client_secret: server_config.client_secret,
            port,
            server_url: server_url.trim_end_matches('/').to_string(),
            token_endpoint: metadata.token_endpoint,
        })
    }

    /// loopback の redirect_uri (`http://127.0.0.1:<port>/callback`)。
    pub(crate) fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/callback", self.port)
    }
}

#[cfg(test)]
impl LoginConfig {
    pub(crate) fn for_test() -> Self {
        let nanos = ::std::time::SystemTime::now()
            .duration_since(::std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self {
            auth_endpoint: format!("https://idp-{nanos}.example.com/auth"),
            client_id: format!("client-{nanos}"),
            client_secret: format!("secret-{nanos}"),
            port: 9787,
            server_url: format!("https://server-{nanos}.example.com"),
            token_endpoint: format!("https://idp-{nanos}.example.com/token"),
        }
    }
}

/// loopback + PKCE でログインし、refresh_token を `TokenStore` へ、
/// 接続先サーバー URL を `ConfigStore` へ保存する。
pub(crate) async fn run(config: LoginConfig) -> ::anyhow::Result<()> {
    let listener = ::tokio::net::TcpListener::bind(("127.0.0.1", config.port)).await?;
    let redirect_uri = config.redirect_uri();
    let authorization =
        build_authorization_request(&config.auth_endpoint, &config.client_id, &redirect_uri)?;

    let browser_open_error = try_open_browser(&authorization.authorization_url)
        .err()
        .map(|e| e.to_string());
    eprintln!(
        "{}",
        browser_open_notice(
            &authorization.authorization_url,
            browser_open_error.as_deref()
        )
    );

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
    ConfigStore::from_env()?.save(&StoredConfig {
        server_url: config.server_url,
    })?;

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

fn browser_open_notice(url: &str, error: Option<&str>) -> String {
    match error {
        Some(error) => format!(
            "Could not open browser automatically: {error}\nOpen this URL in your browser to authorize:\n\n{url}\n"
        ),
        None => format!(
            "Opened browser automatically. If your browser did not open, use this URL:\n\n{url}\n"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::spawn_json_server;

    #[::tokio::test]
    async fn fetch_builds_config_from_server_config_and_discovery() -> ::anyhow::Result<()> {
        let (issuer, idp) = spawn_json_server(
            "200 OK",
            r#"{"authorization_endpoint":"https://idp.example.com/auth","token_endpoint":"https://idp.example.com/token"}"#
                .to_string(),
        )
        .await?;
        let (server_url, server) = spawn_json_server(
            "200 OK",
            format!(r#"{{"client_id":"cid","client_secret":"sec","issuer":"{issuer}"}}"#),
        )
        .await?;

        // 末尾スラッシュ付きで渡しても正規化されて保存される
        let config = LoginConfig::fetch(&format!("{server_url}/"), 9787).await?;
        server.await??;
        idp.await??;

        assert_eq!(config.auth_endpoint, "https://idp.example.com/auth");
        assert_eq!(config.client_id, "cid");
        assert_eq!(config.client_secret, "sec");
        assert_eq!(config.port, 9787);
        assert_eq!(config.server_url, server_url);
        assert_eq!(config.token_endpoint, "https://idp.example.com/token");
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_errors_when_server_config_is_unavailable() -> ::anyhow::Result<()> {
        let (server_url, server) = spawn_json_server("404 Not Found", "".to_string()).await?;
        let result = LoginConfig::fetch(&server_url, 9787).await;
        server.await??;
        let error = result
            .err()
            .ok_or_else(|| ::anyhow::anyhow!("expected fetch to return an error"))?;
        assert!(error.to_string().contains("404"));
        Ok(())
    }

    #[test]
    fn redirect_uri_uses_loopback_and_port() {
        let config = LoginConfig {
            port: 12345,
            ..LoginConfig::for_test()
        };
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

    #[test]
    fn browser_open_notice_for_success_mentions_auto_open_and_url() {
        let message = browser_open_notice("https://example.com", None);
        assert!(message.contains("Opened browser automatically"));
        assert!(message.contains("https://example.com"));
    }

    #[test]
    fn browser_open_notice_for_failure_mentions_manual_open_and_error() {
        let message = browser_open_notice("https://example.com", Some("spawn failed"));
        assert!(message.contains("Could not open browser automatically"));
        assert!(message.contains("spawn failed"));
        assert!(message.contains("https://example.com"));
    }
}
