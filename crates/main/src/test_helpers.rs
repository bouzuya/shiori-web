pub(crate) struct MockOidcClient {
    sub: String,
}

impl MockOidcClient {
    pub(crate) fn new(sub: impl Into<String>) -> Self {
        Self { sub: sub.into() }
    }
}

#[async_trait::async_trait]
impl crate::extractor::OidcClient for MockOidcClient {
    fn build_authentication_request(&self) -> crate::extractor::AuthenticationRequest {
        crate::extractor::AuthenticationRequest {
            nonce: "test_nonce".to_string(),
            state: "test_state".to_string(),
            url: "https://provider.example.com/authorize?client_id=test".to_string(),
        }
    }

    async fn exchange_code(
        &self,
        _code: &str,
        _nonce: &str,
    ) -> anyhow::Result<crate::extractor::OidcClaims> {
        Ok(crate::extractor::OidcClaims {
            sub: self.sub.clone(),
        })
    }
}

pub(crate) fn extract_cookies(response: &axum::response::Response<axum::body::Body>) -> String {
    response
        .headers()
        .get_all(axum::http::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
        .collect::<Vec<_>>()
        .join("; ")
}

pub(crate) async fn send_request(
    router: axum::Router<()>,
    request: axum::http::Request<axum::body::Body>,
) -> anyhow::Result<axum::response::Response<axum::body::Body>> {
    let response = tower::ServiceExt::oneshot(router, request).await?;
    Ok(response)
}

pub(crate) trait ResponseExt {
    async fn into_body_string(self) -> anyhow::Result<String>;
}

impl ResponseExt for axum::response::Response<axum::body::Body> {
    async fn into_body_string(self) -> anyhow::Result<String> {
        let bytes = axum::body::to_bytes(self.into_body(), usize::MAX).await?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }
}
