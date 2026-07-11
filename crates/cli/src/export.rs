use crate::ConfigStore;
use crate::TokenStore;
use crate::fetch_provider_metadata;
use crate::fetch_server_config;

pub(crate) struct ExportConfig {
    client_id: String,
    client_secret: String,
    export_url: String,
    token_endpoint: String,
}

impl ExportConfig {
    /// `ConfigStore` に保存された server_url を基点に設定を解決する。
    /// login 未実行 (server_url 未保存) の場合はエラーにする。
    pub(crate) async fn resolve() -> ::anyhow::Result<Self> {
        let stored = ConfigStore::from_env()?.load()?.ok_or_else(|| {
            ::anyhow::anyhow!("no server configured. run `shiori login <SERVER_URL>` first")
        })?;
        Self::fetch(&stored.server_url).await
    }

    /// サーバーの `/cli/config` と issuer の OIDC Discovery から設定を組み立てる。
    /// export URL は `{server_url}/export` に固定する。
    async fn fetch(server_url: &str) -> ::anyhow::Result<Self> {
        let server_config = fetch_server_config(server_url).await?;
        let metadata = fetch_provider_metadata(&server_config.issuer).await?;
        Ok(Self {
            client_id: server_config.client_id,
            client_secret: server_config.client_secret,
            export_url: format!("{}/export", server_url.trim_end_matches('/')),
            token_endpoint: metadata.token_endpoint,
        })
    }
}

#[derive(::serde::Deserialize)]
struct RefreshTokenResponse {
    id_token: String,
}

#[derive(::serde::Serialize)]
struct TokenRefresh<'a> {
    client_id: &'a str,
    client_secret: &'a str,
    grant_type: &'a str,
    refresh_token: &'a str,
}

pub(crate) async fn run(config: ExportConfig) -> ::anyhow::Result<()> {
    let store = TokenStore::from_env()?;
    let stored = store
        .load()?
        .ok_or_else(|| ::anyhow::anyhow!("not logged in. run `shiori login <SERVER_URL>` first"))?;

    let id_token = refresh_id_token(&config, &stored.refresh_token).await?;
    let response = ::reqwest::Client::new()
        .get(&config.export_url)
        .bearer_auth(id_token)
        .send()
        .await
        .map_err(|e| export_transport_error_message(&config.export_url, &e.to_string()))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return export_error_message(status, &config.export_url, &body);
    }

    print!("{body}");
    Ok(())
}

fn export_error_message(
    status: ::reqwest::StatusCode,
    url: &str,
    body: &str,
) -> ::anyhow::Result<()> {
    if status == ::reqwest::StatusCode::UNAUTHORIZED || status == ::reqwest::StatusCode::FORBIDDEN {
        ::anyhow::bail!(
            "export request to {url} failed with {status}. run `shiori login` and retry"
        );
    }
    ::anyhow::bail!("export request to {url} failed with {status}: {body}");
}

fn export_transport_error_message(url: &str, detail: &str) -> ::anyhow::Error {
    ::anyhow::anyhow!(
        "failed to call export endpoint {url}: {detail}; is the server running and URL correct?"
    )
}

async fn refresh_id_token(config: &ExportConfig, refresh_token: &str) -> ::anyhow::Result<String> {
    let response = ::reqwest::Client::new()
        .post(&config.token_endpoint)
        .form(&TokenRefresh {
            client_id: &config.client_id,
            client_secret: &config.client_secret,
            grant_type: "refresh_token",
            refresh_token,
        })
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        if body.contains("invalid_grant") || body.contains("invalid_request") {
            ::anyhow::bail!("failed to refresh id token ({status}). run `shiori login` again");
        }
        ::anyhow::bail!("failed to refresh id token ({status}): {body}");
    }

    let parsed: RefreshTokenResponse = ::serde_json::from_str(&body)?;
    Ok(parsed.id_token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::spawn_json_server;

    fn for_test_config(export_url: String, token_endpoint: String) -> ExportConfig {
        ExportConfig {
            client_id: "cid".to_string(),
            client_secret: "sec".to_string(),
            export_url,
            token_endpoint,
        }
    }

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

        // 末尾スラッシュ付きの server_url でも export URL は正規化される
        let config = ExportConfig::fetch(&format!("{server_url}/")).await?;
        server.await??;
        idp.await??;

        assert_eq!(config.client_id, "cid");
        assert_eq!(config.client_secret, "sec");
        assert_eq!(config.export_url, format!("{server_url}/export"));
        assert_eq!(config.token_endpoint, "https://idp.example.com/token");
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_errors_when_server_config_is_unavailable() -> ::anyhow::Result<()> {
        let (server_url, server) = spawn_json_server("404 Not Found", "".to_string()).await?;
        let result = ExportConfig::fetch(&server_url).await;
        server.await??;
        let error = result
            .err()
            .ok_or_else(|| ::anyhow::anyhow!("expected fetch to return an error"))?;
        assert!(error.to_string().contains("404"));
        Ok(())
    }

    #[::tokio::test]
    async fn refresh_id_token_returns_id_token() -> ::anyhow::Result<()> {
        let (token_endpoint, server) =
            spawn_json_server("200 OK", r#"{"id_token":"idt"}"#.to_string()).await?;
        let config = for_test_config("http://127.0.0.1:1/export".to_string(), token_endpoint);

        let id_token = refresh_id_token(&config, "rt").await?;
        server.await??;

        assert_eq!(id_token, "idt");
        Ok(())
    }

    #[::tokio::test]
    async fn refresh_id_token_invalid_grant_prompts_relogin() -> ::anyhow::Result<()> {
        let (token_endpoint, server) = spawn_json_server(
            "400 Bad Request",
            r#"{"error":"invalid_grant"}"#.to_string(),
        )
        .await?;
        let config = for_test_config("http://127.0.0.1:1/export".to_string(), token_endpoint);

        let error = refresh_id_token(&config, "rt")
            .await
            .err()
            .ok_or_else(|| ::anyhow::anyhow!("expected refresh_id_token to return an error"))?;
        server.await??;

        let message = error.to_string();
        assert!(message.contains("run `shiori login` again"));
        Ok(())
    }

    #[test]
    fn export_unauthorized_error_prompts_relogin() -> ::anyhow::Result<()> {
        let error = export_error_message(
            ::reqwest::StatusCode::UNAUTHORIZED,
            "http://127.0.0.1:3000/export",
            "",
        )
        .err()
        .ok_or_else(|| ::anyhow::anyhow!("expected error"))?;
        assert!(error.to_string().contains("run `shiori login`"));
        assert!(error.to_string().contains("http://127.0.0.1:3000/export"));
        Ok(())
    }

    #[test]
    fn export_forbidden_error_prompts_relogin() -> ::anyhow::Result<()> {
        let error = export_error_message(
            ::reqwest::StatusCode::FORBIDDEN,
            "http://127.0.0.1:3000/export",
            "",
        )
        .err()
        .ok_or_else(|| ::anyhow::anyhow!("expected error"))?;
        assert!(error.to_string().contains("run `shiori login`"));
        assert!(error.to_string().contains("http://127.0.0.1:3000/export"));
        Ok(())
    }

    #[test]
    fn export_transport_error_mentions_endpoint_and_hint() {
        let error = export_transport_error_message("http://127.0.0.1:3000/export", "boom");
        let message = error.to_string();
        assert!(message.contains("http://127.0.0.1:3000/export"));
        assert!(message.contains("is the server running"));
    }
}
