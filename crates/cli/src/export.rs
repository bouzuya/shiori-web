use crate::TokenStore;

const DEFAULT_EXPORT_URL: &str = "http://localhost:3000/export";
const EMBEDDED_EXPORT_URL: Option<&str> = option_env!("SHIORI_EXPORT_URL");
// Step 5 (export 再構成) で ConfigStore + /cli/config 参照に置き換えて削除する暫定措置。
const EMBEDDED_CLIENT_ID: Option<&str> = option_env!("SHIORI_OIDC_CLIENT_ID");
const EMBEDDED_CLIENT_SECRET: Option<&str> = option_env!("SHIORI_OIDC_CLIENT_SECRET");

pub(crate) struct ExportConfig {
    client_id: String,
    client_secret: String,
    export_url: String,
    token_endpoint: String,
}

impl ExportConfig {
    pub(crate) fn default_with(export_url: Option<String>) -> ::anyhow::Result<Self> {
        let client_id = EMBEDDED_CLIENT_ID.ok_or_else(|| {
            ::anyhow::anyhow!("this binary was built without SHIORI_OIDC_CLIENT_ID")
        })?;
        let client_secret = EMBEDDED_CLIENT_SECRET.ok_or_else(|| {
            ::anyhow::anyhow!("this binary was built without SHIORI_OIDC_CLIENT_SECRET")
        })?;
        let export_url = export_url.unwrap_or_else(|| {
            EMBEDDED_EXPORT_URL
                .unwrap_or(DEFAULT_EXPORT_URL)
                .to_string()
        });
        let export_url = validate_export_url(&export_url)?;
        Ok(Self {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            export_url,
            token_endpoint: "https://oauth2.googleapis.com/token".to_string(),
        })
    }
}

fn validate_export_url(value: &str) -> ::anyhow::Result<String> {
    let url = ::url::Url::parse(value)
        .map_err(|e| ::anyhow::anyhow!("invalid export URL `{value}`: {e}"))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        ::anyhow::bail!("invalid export URL `{value}`: scheme must be http or https");
    }
    Ok(value.to_string())
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

    #[test]
    fn export_config_rejects_invalid_url() {
        let result = ExportConfig::default_with(Some("not-a-url".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn export_config_accepts_http_url() -> ::anyhow::Result<()> {
        let config = ExportConfig::default_with(Some("http://127.0.0.1:3000/export".to_string()))?;
        assert_eq!(config.export_url, "http://127.0.0.1:3000/export");
        Ok(())
    }

    #[test]
    fn export_config_rejects_non_http_scheme() {
        let result = ExportConfig::default_with(Some("ftp://127.0.0.1/export".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn default_export_url_matches_compile_time_env() {
        assert_eq!(EMBEDDED_EXPORT_URL, option_env!("SHIORI_EXPORT_URL"));
    }
}
