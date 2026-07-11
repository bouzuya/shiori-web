mod config_store;
mod discovery;
mod export;
mod login;
mod loopback;
mod oidc;
mod server_config;
#[cfg(test)]
mod test_helpers;
mod token_store;

pub(crate) use self::config_store::ConfigStore;
pub(crate) use self::config_store::StoredConfig;
pub(crate) use self::discovery::fetch_provider_metadata;
pub(crate) use self::export::ExportConfig;
pub(crate) use self::export::run as run_export;
pub(crate) use self::login::LoginConfig;
pub(crate) use self::login::run;
pub(crate) use self::loopback::receive_callback;
pub(crate) use self::oidc::TokenExchange;
pub(crate) use self::oidc::build_authorization_request;
pub(crate) use self::oidc::exchange_code;
pub(crate) use self::server_config::fetch_server_config;
pub(crate) use self::token_store::StoredToken;
pub(crate) use self::token_store::TokenStore;

#[derive(::clap::Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(::clap::Subcommand)]
enum Subcommand {
    /// login 済みサーバーからブックマークを NDJSON としてエクスポートし stdout へ出す。
    Export,
    /// 指定サーバーの OIDC (loopback + PKCE) でログインし、トークンと接続先をローカルに保存する。
    Login(LoginArgs),
}

#[derive(::clap::Args)]
struct LoginArgs {
    #[arg(default_value_t = 9787, env = "SHIORI_LOOPBACK_PORT", long)]
    port: u16,
    /// 接続先の shiori サーバー URL (例: https://shiori.example.com)
    server_url: String,
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    match <Cli as ::clap::Parser>::parse().subcommand {
        Subcommand::Export => run_export(ExportConfig::resolve().await?).await,
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
        let Subcommand::Export = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected export subcommand"));
        };
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
}
