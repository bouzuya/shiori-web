/// OIDC Discovery (`{issuer}/.well-known/openid-configuration`) の応答のうち、
/// CLI が使うエンドポイントのみ (未知フィールドは無視)。
// login / export が消費するまで bin では未使用。
#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq, ::serde::Deserialize)]
pub(crate) struct ProviderMetadata {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
}

fn discovery_url(issuer: &str) -> String {
    format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    )
}

/// issuer から OIDC Discovery でプロバイダーのエンドポイントを取得する。
// login / export が消費するまで bin では未使用。
#[allow(dead_code)]
pub(crate) async fn fetch_provider_metadata(issuer: &str) -> ::anyhow::Result<ProviderMetadata> {
    let url = discovery_url(issuer);
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

    #[test]
    fn discovery_url_appends_well_known_path() {
        assert_eq!(
            discovery_url("https://accounts.google.com"),
            "https://accounts.google.com/.well-known/openid-configuration"
        );
    }

    #[test]
    fn discovery_url_trims_trailing_slash() {
        assert_eq!(
            discovery_url("https://accounts.google.com/"),
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

    /// 1接続を受けて固定の HTTP 応答を返すモック。
    async fn spawn_json_server(
        status_line: &'static str,
        body: String,
    ) -> ::anyhow::Result<(String, ::tokio::task::JoinHandle<::anyhow::Result<()>>)> {
        let listener = ::tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
        let url = format!("http://{}", listener.local_addr()?);
        let handle = ::tokio::spawn(async move {
            let (mut stream, _peer) = listener.accept().await?;
            let (read_half, mut write_half) = stream.split();
            let mut reader = ::tokio::io::BufReader::new(read_half);

            loop {
                let mut line = String::new();
                let read = ::tokio::io::AsyncBufReadExt::read_line(&mut reader, &mut line).await?;
                if read == 0 || line.trim_end().is_empty() {
                    break;
                }
            }

            let response = format!(
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            ::tokio::io::AsyncWriteExt::write_all(&mut write_half, response.as_bytes()).await?;
            ::tokio::io::AsyncWriteExt::flush(&mut write_half).await?;
            Ok(())
        });
        Ok((url, handle))
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
