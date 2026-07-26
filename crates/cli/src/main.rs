mod bookmark;
mod oidc;
mod oidc_client_secrets;
mod server;
mod storage;
mod subcommand;
#[cfg(test)]
mod test_helpers;

pub(crate) use self::bookmark::CachedBookmark;
pub(crate) use self::bookmark::max_updated_at;
pub(crate) use self::bookmark::merge_bookmarks;
pub(crate) use self::bookmark::sort_bookmarks;
pub(crate) use self::oidc::RefreshTokenResponse;
pub(crate) use self::oidc::TokenExchange;
pub(crate) use self::oidc::TokenRefresh;
pub(crate) use self::oidc::build_authorization_request;
pub(crate) use self::oidc::exchange_code;
pub(crate) use self::oidc::fetch_provider_metadata;
pub(crate) use self::oidc::receive_callback;
pub(crate) use self::oidc_client_secrets::fetch_oidc_client_secrets;
pub(crate) use self::server::ExportConfig;
pub(crate) use self::server::fetch_export;
pub(crate) use self::server::refresh_id_token;
pub(crate) use self::storage::ConfigStore;
pub(crate) use self::storage::ExportCache;
pub(crate) use self::storage::StoredConfig;
pub(crate) use self::storage::StoredToken;
pub(crate) use self::storage::TokenStore;
pub(crate) use self::subcommand::Subcommand;

#[derive(::clap::Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    <Cli as ::clap::Parser>::parse().subcommand.execute().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_login_subcommand_with_default_port() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from([
            "shiori",
            "login",
            "https://shiori.example.com",
        ])?;
        let Subcommand::Login(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected login subcommand"));
        };
        assert_eq!(args.port, 9787);
        assert_eq!(args.server_url, "https://shiori.example.com");
        Ok(())
    }

    #[test]
    fn parses_login_subcommand_port_override() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from([
            "shiori",
            "login",
            "--port",
            "5555",
            "https://shiori.example.com",
        ])?;
        let Subcommand::Login(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected login subcommand"));
        };
        assert_eq!(args.port, 5555);
        Ok(())
    }

    #[test]
    fn login_requires_server_url() {
        assert!(<Cli as ::clap::Parser>::try_parse_from(["shiori", "login"]).is_err());
    }

    #[test]
    fn parses_export_subcommand() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "export"])?;
        let Subcommand::Export(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected export subcommand"));
        };
        assert!(!args.refresh);
        Ok(())
    }

    #[test]
    fn parses_export_subcommand_with_refresh_flag() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "export", "--refresh"])?;
        let Subcommand::Export(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected export subcommand"));
        };
        assert!(args.refresh);
        Ok(())
    }

    #[test]
    fn export_rejects_extra_arguments() {
        assert!(
            <Cli as ::clap::Parser>::try_parse_from(["shiori", "export", "--url", "http://x"])
                .is_err()
        );
    }

    #[test]
    fn requires_a_subcommand() {
        assert!(<Cli as ::clap::Parser>::try_parse_from(["shiori"]).is_err());
    }

    #[test]
    fn version_flag_reports_crate_version() -> ::anyhow::Result<()> {
        let error = <Cli as ::clap::Parser>::try_parse_from(["shiori", "--version"])
            .err()
            .ok_or_else(|| ::anyhow::anyhow!("expected --version to exit"))?;
        assert_eq!(error.kind(), ::clap::error::ErrorKind::DisplayVersion);
        assert!(error.to_string().contains(::std::env!("CARGO_PKG_VERSION")));
        Ok(())
    }
}
