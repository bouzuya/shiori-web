use crate::CachedBookmark;
use crate::ConfigStore;
use crate::RefreshTokenResponse;
use crate::TokenRefresh;
use crate::fetch_oidc_client_secrets;
use crate::fetch_provider_metadata;

pub(crate) struct ExportParams {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) export_url: String,
    pub(crate) token_endpoint: String,
}

impl ExportParams {
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
    pub(crate) async fn fetch(server_url: &str) -> ::anyhow::Result<Self> {
        let secrets = fetch_oidc_client_secrets(server_url).await?;
        let metadata = fetch_provider_metadata(&secrets.issuer).await?;
        Ok(Self {
            client_id: secrets.client_id,
            client_secret: secrets.client_secret,
            export_url: format!("{}/export", server_url.trim_end_matches('/')),
            token_endpoint: metadata.token_endpoint,
        })
    }
}

/// export エンドポイントに GET し、NDJSON を parse して返す。
/// `since` が `Some` のとき `?since=` クエリを付けて差分だけを取得する。
pub(crate) async fn fetch_export(
    ExportParams {
        client_id: _,
        client_secret: _,
        export_url,
        token_endpoint: _,
    }: &ExportParams,
    id_token: &str,
    since: Option<&str>,
) -> ::anyhow::Result<Vec<CachedBookmark>> {
    let mut request = ::reqwest::Client::new()
        .get(export_url)
        .bearer_auth(id_token);
    if let Some(since) = since {
        request = request.query(&[("since", since)]);
    }
    let response = request
        .send()
        .await
        .map_err(|e| build_export_transport_error(export_url, &e.to_string()))?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        return Err(build_export_error(status, export_url, &body));
    }
    body.lines()
        .filter(|l| !l.is_empty())
        .map(CachedBookmark::parse)
        .collect()
}

fn build_export_error(status: ::reqwest::StatusCode, url: &str, body: &str) -> ::anyhow::Error {
    if status == ::reqwest::StatusCode::UNAUTHORIZED || status == ::reqwest::StatusCode::FORBIDDEN {
        return ::anyhow::anyhow!(
            "export request to {url} failed with {status}. run `shiori login` and retry"
        );
    }
    ::anyhow::anyhow!("export request to {url} failed with {status}: {body}")
}

fn build_export_transport_error(url: &str, detail: &str) -> ::anyhow::Error {
    ::anyhow::anyhow!(
        "failed to call export endpoint {url}: {detail}; is the server running and URL correct?"
    )
}

pub(crate) async fn refresh_id_token(
    ExportParams {
        client_id,
        client_secret,
        export_url: _,
        token_endpoint,
    }: &ExportParams,
    refresh_token: &str,
) -> ::anyhow::Result<String> {
    let response = ::reqwest::Client::new()
        .post(token_endpoint)
        .form(&TokenRefresh {
            client_id,
            client_secret,
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

    #[test]
    fn export_unauthorized_error_prompts_relogin() {
        let error = build_export_error(
            ::reqwest::StatusCode::UNAUTHORIZED,
            "http://127.0.0.1:3000/export",
            "",
        );
        assert!(error.to_string().contains("run `shiori login`"));
        assert!(error.to_string().contains("http://127.0.0.1:3000/export"));
    }

    #[test]
    fn export_forbidden_error_prompts_relogin() {
        let error = build_export_error(
            ::reqwest::StatusCode::FORBIDDEN,
            "http://127.0.0.1:3000/export",
            "",
        );
        assert!(error.to_string().contains("run `shiori login`"));
        assert!(error.to_string().contains("http://127.0.0.1:3000/export"));
    }

    #[test]
    fn export_transport_error_mentions_endpoint_and_hint() {
        let error = build_export_transport_error("http://127.0.0.1:3000/export", "boom");
        let message = error.to_string();
        assert!(message.contains("http://127.0.0.1:3000/export"));
        assert!(message.contains("is the server running"));
    }
}
