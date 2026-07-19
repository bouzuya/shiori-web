use crate::AppState;

pub(crate) fn router() -> ::axum::Router<AppState> {
    ::axum::Router::new().route("/cli/config", ::axum::routing::get(get_oidc_client_secrets))
}

#[derive(::serde::Serialize)]
struct OidcClientSecretsResponse {
    client_id: String,
    client_secret: String,
    issuer: String,
}

async fn get_oidc_client_secrets(
    ::axum::extract::State(state): ::axum::extract::State<AppState>,
) -> ::axum::Json<OidcClientSecretsResponse> {
    ::axum::Json(OidcClientSecretsResponse {
        client_id: state.oidc_client_secrets.client_id,
        client_secret: state.oidc_client_secrets.client_secret,
        issuer: state.oidc_client_secrets.issuer,
    })
}

#[cfg(test)]
mod tests {
    use crate::AppState;
    use crate::OidcClientSecrets;
    use crate::test_helpers::MockAuthorizationCodeClient;
    use crate::test_helpers::ResponseExt as _;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_repo;
    use crate::test_helpers::firestore_user_settings_reader;
    use crate::test_helpers::firestore_user_settings_repository;
    use crate::test_helpers::send_request;

    #[::tokio::test]
    #[::serial_test::serial]
    async fn get_oidc_client_secrets_returns_client_credentials_and_issuer() -> ::anyhow::Result<()>
    {
        let oidc_client_secrets = OidcClientSecrets::for_test();
        let state = AppState::new(
            "".to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            oidc_client_secrets.clone(),
            TEST_COOKIE_SIGNING_SECRET,
            crate::test_helpers::mock_id_token_verifier(),
            ::std::sync::Arc::new(MockAuthorizationCodeClient::new("cli_config_user")),
            firestore_user_repo()?,
            firestore_user_settings_reader()?,
            firestore_user_settings_repository()?,
        );
        let response = send_request(
            crate::router::router("").with_state(state),
            ::axum::http::Request::builder()
                .uri("/cli/config")
                .body(::axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(response.status(), ::axum::http::StatusCode::OK);
        let content_type = response
            .headers()
            .get(::axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("application/json"),
            "expected application/json, got: {content_type}"
        );
        let body = response.into_body_string().await?;
        let value: ::serde_json::Value = ::serde_json::from_str(&body)?;
        assert_eq!(
            value,
            ::serde_json::json!({
                "client_id": oidc_client_secrets.client_id,
                "client_secret": oidc_client_secrets.client_secret,
                "issuer": oidc_client_secrets.issuer,
            })
        );
        Ok(())
    }
}
