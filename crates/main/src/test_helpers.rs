use kernel::BookmarkReader;
use kernel::BookmarkRepository;
use kernel::GoogleUserId;
use kernel::User;
use kernel::UserId;
use kernel::UserRepository;
use kernel::UserSettingsReader;
use kernel::UserSettingsRepository;

use crate::FirestoreBookmarkReader;
use crate::FirestoreBookmarkRepository;
use crate::FirestoreUserRepository;
use crate::FirestoreUserSettingsReader;
use crate::FirestoreUserSettingsRepository;

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

pub(crate) struct MockUserRepository {
    users: std::sync::Mutex<Vec<User>>,
}

impl MockUserRepository {
    pub(crate) fn new() -> Self {
        Self {
            users: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl UserRepository for MockUserRepository {
    async fn find(&self, id: &UserId) -> anyhow::Result<Option<User>> {
        let users = self.users.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(users.iter().find(|u| u.id() == *id).cloned())
    }

    async fn find_by_google_user_id(&self, id: &GoogleUserId) -> anyhow::Result<Option<User>> {
        let users = self.users.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(users.iter().find(|u| u.google_user_id() == id).cloned())
    }

    async fn store(&self, user: User) -> anyhow::Result<()> {
        let mut users = self.users.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        if let Some(pos) = users.iter().position(|u| u.id() == user.id()) {
            users[pos] = user;
        } else {
            users.push(user);
        }
        Ok(())
    }
}

pub(crate) const TEST_COOKIE_SIGNING_SECRET: &str =
    "test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding";

pub(crate) fn unique_user_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("u{nanos}")
}

pub(crate) fn firestore() -> anyhow::Result<bouzuya_firestore_client::Firestore> {
    Ok(bouzuya_firestore_client::Firestore::new(
        bouzuya_firestore_client::FirestoreOptions::default(),
    )?)
}

pub(crate) fn firestore_user_repo() -> anyhow::Result<std::sync::Arc<dyn UserRepository>> {
    Ok(std::sync::Arc::new(FirestoreUserRepository::new(
        firestore()?,
    )))
}

pub(crate) fn firestore_user_settings_reader()
-> anyhow::Result<std::sync::Arc<dyn UserSettingsReader>> {
    Ok(std::sync::Arc::new(FirestoreUserSettingsReader::new(
        firestore()?,
    )))
}

pub(crate) fn firestore_user_settings_repository()
-> anyhow::Result<std::sync::Arc<dyn UserSettingsRepository>> {
    Ok(std::sync::Arc::new(FirestoreUserSettingsRepository::new(
        firestore()?,
    )))
}

pub(crate) fn firestore_bookmark_reader() -> anyhow::Result<std::sync::Arc<dyn BookmarkReader>> {
    Ok(std::sync::Arc::new(FirestoreBookmarkReader::new(
        firestore()?,
    )))
}

pub(crate) fn firestore_bookmark_repo() -> anyhow::Result<std::sync::Arc<dyn BookmarkRepository>> {
    Ok(std::sync::Arc::new(FirestoreBookmarkRepository::new(
        firestore()?,
    )))
}

pub(crate) fn test_app(sub: impl Into<String>) -> anyhow::Result<axum::Router> {
    let state = crate::AppState::new(
        "".to_string(),
        firestore_bookmark_reader()?,
        firestore_bookmark_repo()?,
        TEST_COOKIE_SIGNING_SECRET,
        std::sync::Arc::new(MockOidcClient::new(sub)),
        firestore_user_repo()?,
        firestore_user_settings_reader()?,
        firestore_user_settings_repository()?,
    );
    Ok(crate::router::router("").with_state(state))
}

pub(crate) fn test_app_with_mock_repo(sub: impl Into<String>) -> anyhow::Result<axum::Router> {
    let bookmark_repository = std::sync::Arc::new(FirestoreBookmarkRepository::new(firestore()?))
        as std::sync::Arc<dyn BookmarkRepository>;
    let state = crate::AppState::new(
        "".to_string(),
        firestore_bookmark_reader()?,
        bookmark_repository,
        TEST_COOKIE_SIGNING_SECRET,
        std::sync::Arc::new(MockOidcClient::new(sub)),
        std::sync::Arc::new(MockUserRepository::new()),
        firestore_user_settings_reader()?,
        firestore_user_settings_repository()?,
    );
    Ok(crate::router::router("").with_state(state))
}

pub(crate) fn form_body<T: serde::Serialize>(data: &T) -> anyhow::Result<axum::body::Body> {
    let encoded = serde_urlencoded::to_string(data)?;
    Ok(axum::body::Body::from(encoded))
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
