use crate::LoginConfig;
use crate::TokenStore;

const DEFAULT_EXPORT_URL: &str = "http://127.0.0.1:3000/export";

pub(crate) struct ExportConfig {
    client_id: String,
    client_secret: String,
    export_url: String,
    token_endpoint: String,
}

impl ExportConfig {
    pub(crate) fn default() -> ::anyhow::Result<Self> {
        let login_config = LoginConfig::google_embedded(0)?;
        Ok(Self {
            client_id: login_config.client_id,
            client_secret: login_config.client_secret,
            export_url: DEFAULT_EXPORT_URL.to_string(),
            token_endpoint: login_config.token_endpoint,
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
        .ok_or_else(|| ::anyhow::anyhow!("not logged in. run `shiori login` first"))?;

    let id_token = refresh_id_token(&config, &stored.refresh_token).await?;
    let response = ::reqwest::Client::new()
        .get(&config.export_url)
        .bearer_auth(id_token)
        .send()
        .await?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        ::anyhow::bail!("export request failed with {status}: {body}");
    }

    print!("{body}");
    Ok(())
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

    fn for_test_config(export_url: String, token_endpoint: String) -> ExportConfig {
        ExportConfig {
            client_id: "cid".to_string(),
            client_secret: "sec".to_string(),
            export_url,
            token_endpoint,
        }
    }

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
}
