/// OIDC Discovery (`{issuer}/.well-known/openid-configuration`) の応答のうち、
/// CLI が使うエンドポイントのみ (未知フィールドは無視)。
#[derive(Clone, Debug, Eq, PartialEq, ::serde::Deserialize)]
pub(crate) struct ProviderMetadata {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
}

fn provider_metadata_url(issuer: &str) -> String {
    format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    )
}

/// issuer から OIDC Discovery でプロバイダーのエンドポイントを取得する。
pub(crate) async fn fetch_provider_metadata(issuer: &str) -> ::anyhow::Result<ProviderMetadata> {
    let url = provider_metadata_url(issuer);
    let response = ::reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| ::anyhow::anyhow!("failed to call {url}: {e}"))?;
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
    fn provider_metadata_url_appends_well_known_path() {
        assert_eq!(
            provider_metadata_url("https://accounts.google.com"),
            "https://accounts.google.com/.well-known/openid-configuration"
        );
    }

    #[test]
    fn provider_metadata_url_trims_trailing_slash() {
        assert_eq!(
            provider_metadata_url("https://accounts.google.com/"),
            "https://accounts.google.com/.well-known/openid-configuration"
        );
    }

    #[test]
    fn deserializes_provider_metadata_ignoring_unknown_fields() -> ::anyhow::Result<()> {
        let json = r#"{
            "issuer": "https://accounts.google.com",
            "authorization_endpoint": "https://accounts.google.com/o/oauth2/v2/auth",
            "token_endpoint": "https://oauth2.googleapis.com/token",
            "jwks_uri": "https://www.googleapis.com/oauth2/v3/certs"
        }"#;
        let metadata: ProviderMetadata = ::serde_json::from_str(json)?;
        assert_eq!(
            metadata,
            ProviderMetadata {
                authorization_endpoint: "https://accounts.google.com/o/oauth2/v2/auth".to_string(),
                token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
            }
        );
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_provider_metadata_returns_endpoints() -> ::anyhow::Result<()> {
        let (url, server) = spawn_json_server(
            "200 OK",
            r#"{"authorization_endpoint":"https://idp.example.com/auth","token_endpoint":"https://idp.example.com/token"}"#
                .to_string(),
        )
        .await?;
        let metadata = fetch_provider_metadata(&url).await?;
        server.await??;
        assert_eq!(
            metadata.authorization_endpoint,
            "https://idp.example.com/auth"
        );
        assert_eq!(metadata.token_endpoint, "https://idp.example.com/token");
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_provider_metadata_errors_on_non_success_status() -> ::anyhow::Result<()> {
        let (url, server) = spawn_json_server("404 Not Found", "".to_string()).await?;
        let result = fetch_provider_metadata(&url).await;
        server.await??;
        let error = result
            .err()
            .ok_or_else(|| ::anyhow::anyhow!("expected fetch_provider_metadata to error"))?;
        assert!(error.to_string().contains("404"));
        Ok(())
    }
}
