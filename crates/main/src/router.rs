mod auth;
mod bookmark;
mod index_css;
mod root;

use axum::Router;

use crate::AppState;

pub(crate) fn router(base_path: &str) -> Router<AppState> {
    let inner = Router::new()
        .merge(auth::router())
        .merge(bookmark::router())
        .merge(index_css::router())
        .merge(root::router())
        .layer(axum::middleware::from_fn(method_override));
    if base_path.is_empty() {
        inner
    } else {
        Router::new().nest(base_path, inner)
    }
}

async fn method_override(
    mut req: axum::http::Request<axum::body::Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    if req.method() == axum::http::Method::POST {
        let override_method = req.uri().query().and_then(|q| {
            q.split('&').find_map(|pair| {
                let (k, v) = pair.split_once('=')?;
                if k == "_method" {
                    Some(v.to_string())
                } else {
                    None
                }
            })
        });
        if let Some(m) = override_method
            && let Ok(method) = m.parse::<axum::http::Method>()
        {
            *req.method_mut() = method;
        }
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::AppState;
    use crate::test_helpers::MockOidcClient;
    use crate::test_helpers::TEST_COOKIE_SIGNING_SECRET;
    use crate::test_helpers::firestore_bookmark_reader;
    use crate::test_helpers::firestore_bookmark_repo;
    use crate::test_helpers::firestore_user_repo;
    use crate::test_helpers::send_request;

    #[tokio::test]
    #[serial_test::serial]
    async fn with_base_path_routes_are_under_base_path() -> anyhow::Result<()> {
        let base_path = "/app";
        let state = AppState::new(
            base_path.to_string(),
            firestore_bookmark_reader()?,
            firestore_bookmark_repo()?,
            TEST_COOKIE_SIGNING_SECRET,
            Arc::new(MockOidcClient::new("base_path_route_user")),
            firestore_user_repo()?,
        );

        // Route exists under base path
        let response = send_request(
            super::router(base_path).with_state(state.clone()),
            axum::http::Request::builder()
                .uri("/app/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::TEMPORARY_REDIRECT,
            "Expected route under base path to exist"
        );

        // Route does NOT exist without base path
        let response = send_request(
            super::router(base_path).with_state(state),
            axum::http::Request::builder()
                .uri("/auth/signup")
                .body(axum::body::Body::empty())?,
        )
        .await?;
        assert_eq!(
            response.status(),
            axum::http::StatusCode::NOT_FOUND,
            "Expected route without base path to return 404"
        );
        Ok(())
    }
}
