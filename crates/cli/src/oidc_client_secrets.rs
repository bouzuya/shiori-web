/// サーバーの `GET /cli/oidc-client-secrets` が返す、CLI 認証用の OIDC クライアント設定。
#[derive(Clone, Debug, Eq, PartialEq, ::serde::Deserialize)]
pub(crate) struct OidcClientSecrets {
    pub client_id: String,
    pub client_secret: String,
    pub issuer: String,
}

fn oidc_client_secrets_url(server_url: &str) -> String {
    format!(
        "{}/cli/oidc-client-secrets",
        server_url.trim_end_matches('/')
    )
}

/// サーバーのベース URL から `/cli/oidc-client-secrets` を取得する。
pub(crate) async fn fetch_oidc_client_secrets(
    server_url: &str,
) -> ::anyhow::Result<OidcClientSecrets> {
    let url = oidc_client_secrets_url(server_url);
    let response = ::reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| {
            ::anyhow::anyhow!("failed to call {url}: {e}; is the server running and URL correct?")
        })?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        ::anyhow::bail!("request to {url} failed with {status}: {body}");
    }
    Ok(::serde_json::from_str(&body)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::spawn_json_server;

    #[test]
    fn oidc_client_secrets_url_appends_path() {
        assert_eq!(
            oidc_client_secrets_url("https://example.com"),
            "https://example.com/cli/oidc-client-secrets"
        );
    }

    #[test]
    fn oidc_client_secrets_url_trims_trailing_slash() {
        assert_eq!(
            oidc_client_secrets_url("https://example.com/app/"),
            "https://example.com/app/cli/oidc-client-secrets"
        );
    }

    #[test]
    fn deserializes_oidc_client_secrets() -> ::anyhow::Result<()> {
        let json =
            r#"{"client_id":"cid","client_secret":"sec","issuer":"https://accounts.google.com"}"#;
        let config: OidcClientSecrets = ::serde_json::from_str(json)?;
        assert_eq!(
            config,
            OidcClientSecrets {
                client_id: "cid".to_string(),
                client_secret: "sec".to_string(),
                issuer: "https://accounts.google.com".to_string(),
            }
        );
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_oidc_client_secrets_returns_secrets() -> ::anyhow::Result<()> {
        let (url, server) = spawn_json_server(
            "200 OK",
            r#"{"client_id":"cid","client_secret":"sec","issuer":"https://accounts.google.com"}"#
                .to_string(),
        )
        .await?;
        let config = fetch_oidc_client_secrets(&url).await?;
        server.await??;
        assert_eq!(config.client_id, "cid");
        assert_eq!(config.client_secret, "sec");
        assert_eq!(config.issuer, "https://accounts.google.com");
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_oidc_client_secrets_errors_on_non_success_status() -> ::anyhow::Result<()> {
        let (url, server) = spawn_json_server("404 Not Found", "".to_string()).await?;
        let result = fetch_oidc_client_secrets(&url).await;
        server.await??;
        let error = result.err().ok_or_else(|| {
            ::anyhow::anyhow!("expected fetch_oidc_client_secrets to return an error")
        })?;
        assert!(error.to_string().contains("404"));
        Ok(())
    }
}
