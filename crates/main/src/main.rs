mod cli;
mod cookie_jar;
mod extractor;
mod firestore;
mod oidc;
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
pub(crate) use self::oidc::AuthenticationRequest;
pub(crate) use self::oidc::AuthorizationCodeClient;
pub(crate) use self::oidc::IdTokenVerifier;
pub(crate) use self::oidc::OidcClaims;
pub(crate) use self::oidc::RealAuthorizationCodeClient;
pub(crate) use self::oidc::RealAuthorizationCodeClientOptions;
pub(crate) use self::oidc::RealIdTokenVerifier;
pub(crate) use self::oidc::RealIdTokenVerifierOptions;
pub(crate) use self::state::AppState;

use crate::cli::Cli;
use crate::cli::ServeArgs;
use crate::cli::Subcommand;

async fn build_state(args: &ServeArgs) -> ::anyhow::Result<AppState> {
    let options = RealAuthorizationCodeClientOptions {
        client_id: args.oidc_client_id.clone(),
        client_secret: args.oidc_client_secret.clone(),
        issuer_url: args.oidc_issuer_url.clone(),
        redirect_uri: args.oidc_redirect_uri.clone(),
    };
    let oidc_client = RealAuthorizationCodeClient::new(options).await?;
    let id_token_verifier = RealIdTokenVerifier::new(RealIdTokenVerifierOptions {
        client_id: args.oidc_cli_client_id.clone(),
        issuer_url: args.oidc_issuer_url.clone(),
    })
    .await?;
    let firestore =
        ::bouzuya_firestore_client::Firestore::new(::bouzuya_firestore_client::FirestoreOptions {
            database_id: Some(args.database_id.clone()),
            project_id: Some(args.project_id.clone()),
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
        args.base_path.clone(),
        bookmark_reader,
        bookmark_repository,
        &args.cookie_signing_secret,
        ::std::sync::Arc::new(id_token_verifier),
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

async fn run_server(args: ServeArgs) -> ::anyhow::Result<()> {
    ::tracing_subscriber::fmt()
        .with_env_filter(
            ::tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| ::tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let state = build_state(&args).await?;
    let listener = ::tokio::net::TcpListener::bind(format!("0.0.0.0:{}", args.port)).await?;
    ::tracing::info!("listening on 0.0.0.0:{}", args.port);
    ::axum::serve(listener, router::router(&args.base_path).with_state(state)).await?;
    Ok(())
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    match <Cli as ::clap::Parser>::parse().subcommand {
        Subcommand::GenerateSecret => {
            println!("{}", generate_secret());
            Ok(())
        }
        Subcommand::Serve(args) => run_server(args).await,
    }
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
