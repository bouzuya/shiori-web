use crate::ExportCache;
use crate::ExportParams;
use crate::TokenStore;
use crate::fetch_export;
use crate::max_updated_at;
use crate::merge_bookmarks;
use crate::refresh_id_token;
use crate::sort_bookmarks;

#[derive(::clap::Args)]
pub(crate) struct ExportArgs {
    /// Discard the local cache and refetch all bookmarks
    /// (required to reflect bookmarks deleted on the server).
    #[arg(long)]
    pub refresh: bool,
}

impl ExportArgs {
    pub(crate) async fn execute(self) -> ::anyhow::Result<()> {
        run(self.refresh).await
    }
}

pub(crate) async fn run(refresh: bool) -> ::anyhow::Result<()> {
    let config = ExportParams::resolve().await?;

    let store = TokenStore::from_env()?;
    let stored = store
        .load()?
        .ok_or_else(|| ::anyhow::anyhow!("not logged in. run `shiori login <SERVER_URL>` first"))?;
    let cache_store = ExportCache::from_env()?;
    let cache = if refresh {
        Vec::new()
    } else {
        cache_store.load()?.unwrap_or_default()
    };
    let since = max_updated_at(&cache).map(str::to_string);

    let id_token = refresh_id_token(&config, &stored.refresh_token).await?;
    let incoming = fetch_export(&config, &id_token, since.as_deref()).await?;

    let mut merged = merge_bookmarks(cache, incoming);
    sort_bookmarks(&mut merged);
    cache_store.save(&merged)?;

    for bookmark in &merged {
        ::std::println!("{}", bookmark.line());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::spawn_json_server;

    fn for_test_config(export_url: String, token_endpoint: String) -> ExportParams {
        ExportParams {
            client_id: "cid".to_string(),
            client_secret: "sec".to_string(),
            export_url,
            token_endpoint,
        }
    }

    #[::tokio::test]
    async fn fetch_builds_config_from_oidc_client_secrets_and_discovery() -> ::anyhow::Result<()> {
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
        let config = ExportParams::fetch(&format!("{server_url}/")).await?;
        server.await??;
        idp.await??;

        assert_eq!(config.client_id, "cid");
        assert_eq!(config.client_secret, "sec");
        assert_eq!(config.export_url, format!("{server_url}/export"));
        assert_eq!(config.token_endpoint, "https://idp.example.com/token");
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_errors_when_oidc_client_secrets_are_unavailable() -> ::anyhow::Result<()> {
        let (server_url, server) = spawn_json_server("404 Not Found", "".to_string()).await?;
        let result = ExportParams::fetch(&server_url).await;
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

    #[::tokio::test]
    async fn fetch_export_returns_parsed_bookmarks() -> ::anyhow::Result<()> {
        let body = format!(
            "{}\n{}\n",
            ::serde_json::json!({
                "comment": "",
                "created_at": "2026-01-01T00:00:00.000Z",
                "id": "a",
                "title": "",
                "updated_at": "2026-01-01T00:00:00.000Z",
                "url": "https://example.com/a",
            }),
            ::serde_json::json!({
                "comment": "",
                "created_at": "2026-01-02T00:00:00.000Z",
                "id": "b",
                "title": "",
                "updated_at": "2026-01-02T00:00:00.000Z",
                "url": "https://example.com/b",
            }),
        );
        let (export_url, server) = spawn_json_server("200 OK", body).await?;
        let config = for_test_config(format!("{export_url}/export"), "http://x".to_string());

        let bookmarks = fetch_export(&config, "idt", None).await?;
        let request_line = server.await??;

        assert_eq!(bookmarks.len(), 2);
        assert_eq!(bookmarks[0].id(), "a");
        assert_eq!(bookmarks[1].id(), "b");
        assert!(request_line.starts_with("GET /export "));
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_export_sends_since_as_query_parameter() -> ::anyhow::Result<()> {
        let (export_url, server) = spawn_json_server("200 OK", String::new()).await?;
        let config = for_test_config(format!("{export_url}/export"), "http://x".to_string());

        let _ = fetch_export(&config, "idt", Some("2026-07-06T23:06:49.751Z")).await?;
        let request_line = server.await??;

        assert!(
            request_line.starts_with("GET /export?since=2026-07-06T23%3A06%3A49.751Z "),
            "unexpected request line: {request_line}"
        );
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_export_returns_empty_vec_for_empty_body() -> ::anyhow::Result<()> {
        let (export_url, server) = spawn_json_server("200 OK", String::new()).await?;
        let config = for_test_config(format!("{export_url}/export"), "http://x".to_string());

        let bookmarks = fetch_export(&config, "idt", None).await?;
        server.await??;

        assert!(bookmarks.is_empty());
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_export_errors_on_unauthorized() -> ::anyhow::Result<()> {
        let (export_url, server) = spawn_json_server("401 Unauthorized", String::new()).await?;
        let config = for_test_config(format!("{export_url}/export"), "http://x".to_string());

        let error = fetch_export(&config, "idt", None)
            .await
            .err()
            .ok_or_else(|| ::anyhow::anyhow!("expected fetch_export to return an error"))?;
        server.await??;

        assert!(error.to_string().contains("run `shiori login`"));
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_export_errors_on_server_error() -> ::anyhow::Result<()> {
        let (export_url, server) =
            spawn_json_server("500 Internal Server Error", "boom".to_string()).await?;
        let config = for_test_config(format!("{export_url}/export"), "http://x".to_string());

        let error = fetch_export(&config, "idt", None)
            .await
            .err()
            .ok_or_else(|| ::anyhow::anyhow!("expected fetch_export to return an error"))?;
        server.await??;

        assert!(error.to_string().contains("500"));
        Ok(())
    }

    #[::tokio::test]
    async fn fetch_export_errors_on_malformed_line() -> ::anyhow::Result<()> {
        let (export_url, server) = spawn_json_server("200 OK", "not json\n".to_string()).await?;
        let config = for_test_config(format!("{export_url}/export"), "http://x".to_string());

        let result = fetch_export(&config, "idt", None).await;
        server.await??;

        assert!(result.is_err());
        Ok(())
    }
}
