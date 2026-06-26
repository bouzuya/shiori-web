mod cli;
mod cookie_jar;
mod env;
mod extractor;
mod firestore;
mod router;
mod state;
#[cfg(test)]
mod test_helpers;

pub(crate) use self::cookie_jar::CookieJar;
pub(crate) use self::firestore::BookmarkDocumentData;
pub(crate) use self::firestore::BookmarksCollection;
pub(crate) use self::firestore::DocumentRef;
pub(crate) use self::firestore::FirestoreBookmarkReader;
pub(crate) use self::firestore::FirestoreBookmarkRepository;
pub(crate) use self::firestore::FirestoreCollection;
pub(crate) use self::firestore::FirestoreCollectionExt;
pub(crate) use self::firestore::FirestoreUserRepository;
pub(crate) use self::firestore::FirestoreUserSettingsReader;
pub(crate) use self::firestore::FirestoreUserSettingsRepository;
pub(crate) use self::firestore::GoogleUserIdDocumentData;
pub(crate) use self::firestore::GoogleUserIdsCollection;
pub(crate) use self::firestore::UserDocumentData;
pub(crate) use self::firestore::UserSettingsCollection;
pub(crate) use self::firestore::UserSettingsDocumentData;
pub(crate) use self::firestore::UsersCollection;
pub(crate) use self::state::AppState;

use crate::extractor::real_oidc_client;

async fn build_state(env: &env::Env) -> ::anyhow::Result<AppState> {
    let options = real_oidc_client::RealOidcClientOptions {
        client_id: env.oidc_client_id.clone(),
        client_secret: env.oidc_client_secret.clone(),
        issuer_url: env.oidc_issuer_url.clone(),
        redirect_uri: env.oidc_redirect_uri.clone(),
    };
    let oidc_client = real_oidc_client::RealOidcClient::new(options).await?;
    let firestore =
        ::bouzuya_firestore_client::Firestore::new(::bouzuya_firestore_client::FirestoreOptions {
            database_id: Some(env.database_id.clone()),
            project_id: Some(env.project_id.clone()),
        })?;
    let bookmark_reader = ::std::sync::Arc::new(FirestoreBookmarkReader::new(firestore.clone()));
    let bookmark_repository =
        ::std::sync::Arc::new(FirestoreBookmarkRepository::new(firestore.clone()));
    let user_settings_reader =
        ::std::sync::Arc::new(FirestoreUserSettingsReader::new(firestore.clone()));
    let user_settings_repository =
        ::std::sync::Arc::new(FirestoreUserSettingsRepository::new(firestore.clone()));
    let user_repository = ::std::sync::Arc::new(FirestoreUserRepository::new(firestore));
    Ok(AppState::new(
        env.base_path.clone(),
        bookmark_reader,
        bookmark_repository,
        &env.cookie_signing_secret,
        ::std::sync::Arc::new(oidc_client),
        user_repository,
        user_settings_reader,
        user_settings_repository,
    ))
}

fn generate_secret() -> String {
    let key = ::axum_extra::extract::cookie::Key::generate();
    key.master().iter().map(|b| format!("{b:02x}")).collect()
}

async fn run_server() -> ::anyhow::Result<()> {
    ::tracing_subscriber::fmt()
        .with_env_filter(
            ::tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| ::tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let env = env::Env::from_env()?;
    let state = build_state(&env).await?;
    let listener = ::tokio::net::TcpListener::bind(format!("0.0.0.0:{}", env.port)).await?;
    ::tracing::info!("listening on 0.0.0.0:{}", env.port);
    ::axum::serve(listener, router::router(&env.base_path).with_state(state)).await?;
    Ok(())
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    let args: Vec<String> = ::std::env::args().collect();
    if args.get(1).map(|s| s.as_str()) == Some("generate-secret") {
        println!("{}", generate_secret());
        return Ok(());
    }
    run_server().await
}

#[cfg(test)]
mod tests {
    #[test]
    fn generate_secret_returns_string_usable_as_cookie_signing_secret() {
        let secret = super::generate_secret();
        // cookie_signing_secret は Key::from() に渡すため、UTF-8 バイト列が 64 バイト以上必要
        assert!(
            secret.len() >= 64,
            "generated secret must be at least 64 bytes, got {}",
            secret.len()
        );
        // 実際に Key::from() で変換できることを確認
        let _ = ::axum_extra::extract::cookie::Key::from(secret.as_bytes());
    }
}
