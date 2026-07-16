mod config_store;
mod discovery;
mod export_cache;
mod loopback;
mod oidc;
mod server_config;
mod subcommand;
#[cfg(test)]
mod test_helpers;
mod token_store;

pub(crate) use self::config_store::ConfigStore;
pub(crate) use self::config_store::StoredConfig;
pub(crate) use self::discovery::fetch_provider_metadata;
pub(crate) use self::export_cache::CachedBookmark;
pub(crate) use self::export_cache::ExportCache;
pub(crate) use self::loopback::receive_callback;
pub(crate) use self::oidc::TokenExchange;
pub(crate) use self::oidc::build_authorization_request;
pub(crate) use self::oidc::exchange_code;
pub(crate) use self::server_config::fetch_server_config;
pub(crate) use self::subcommand::Subcommand;
pub(crate) use self::subcommand::export::ExportConfig;
pub(crate) use self::subcommand::export::run as run_export;
pub(crate) use self::subcommand::login::LoginConfig;
pub(crate) use self::subcommand::login::run;
pub(crate) use self::token_store::StoredToken;
pub(crate) use self::token_store::TokenStore;

#[derive(::clap::Parser)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    match <Cli as ::clap::Parser>::parse().subcommand {
        Subcommand::Export(args) => run_export(ExportConfig::resolve().await?, args.refresh).await,
        Subcommand::Login(args) => {
            run(LoginConfig::fetch(&args.server_url, args.port).await?).await
        }
    }
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
