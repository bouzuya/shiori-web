mod login;
mod loopback;
mod oidc;
mod token_store;

pub(crate) use self::login::LoginConfig;
pub(crate) use self::login::run;
pub(crate) use self::loopback::receive_callback;
pub(crate) use self::oidc::TokenExchange;
pub(crate) use self::oidc::build_authorization_request;
pub(crate) use self::oidc::exchange_code;
pub(crate) use self::token_store::StoredToken;
pub(crate) use self::token_store::TokenStore;

#[derive(::clap::Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Subcommand,
}

#[derive(::clap::Subcommand)]
enum Subcommand {
    /// 保存済みトークンでブックマークを NDJSON としてエクスポートし stdout へ出す。
    Export,
    /// Google OIDC (loopback + PKCE) でログインし、トークンをローカルに保存する。
    Login(LoginArgs),
}

#[derive(::clap::Args)]
struct LoginArgs {
    #[arg(env = "OIDC_CLI_CLIENT_ID", long)]
    client_id: String,
    #[arg(env = "OIDC_CLI_CLIENT_SECRET", long)]
    client_secret: String,
    #[arg(default_value_t = 9787, env = "SHIORI_LOOPBACK_PORT", long)]
    port: u16,
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    match <Cli as ::clap::Parser>::parse().subcommand {
        Subcommand::Export => ::anyhow::bail!("export is not yet implemented"),
        Subcommand::Login(args) => {
            run(LoginConfig::google(
                args.client_id,
                args.client_secret,
                args.port,
            ))
            .await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_login_subcommand_with_credentials() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from([
            "shiori",
            "login",
            "--client-id",
            "cid",
            "--client-secret",
            "sec",
        ])?;
        let Subcommand::Login(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected login subcommand"));
        };
        assert_eq!(args.client_id, "cid");
        assert_eq!(args.client_secret, "sec");
        assert_eq!(args.port, 9787);
        Ok(())
    }

    #[test]
    fn parses_export_subcommand() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "export"])?;
        assert!(matches!(cli.subcommand, Subcommand::Export));
        Ok(())
    }

    #[test]
    fn requires_a_subcommand() {
        assert!(<Cli as ::clap::Parser>::try_parse_from(["shiori"]).is_err());
    }
}
