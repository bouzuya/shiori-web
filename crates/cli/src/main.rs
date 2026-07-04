mod export;
mod login;
mod loopback;
mod oidc;
mod token_store;

pub(crate) use self::export::ExportConfig;
pub(crate) use self::export::run as run_export;
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
    Export(ExportArgs),
    /// Google OIDC (loopback + PKCE) でログインし、トークンをローカルに保存する。
    Login(LoginArgs),
}

#[derive(::clap::Args)]
struct ExportArgs {
    #[arg(env = "SHIORI_EXPORT_URL", long)]
    url: Option<String>,
}

#[derive(::clap::Args)]
struct LoginArgs {
    #[arg(default_value_t = 9787, env = "SHIORI_LOOPBACK_PORT", long)]
    port: u16,
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    match <Cli as ::clap::Parser>::parse().subcommand {
        Subcommand::Export(args) => run_export(ExportConfig::default_with(args.url)?).await,
        Subcommand::Login(args) => run(LoginConfig::google_embedded(args.port)?).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_login_subcommand_with_default_port() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "login"])?;
        let Subcommand::Login(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected login subcommand"));
        };
        assert_eq!(args.port, 9787);
        Ok(())
    }

    #[test]
    fn parses_login_subcommand_port_override() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "login", "--port", "5555"])?;
        let Subcommand::Login(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected login subcommand"));
        };
        assert_eq!(args.port, 5555);
        Ok(())
    }

    #[test]
    fn parses_export_subcommand() -> ::anyhow::Result<()> {
        ::temp_env::with_var("SHIORI_EXPORT_URL", None::<&str>, || {
            let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "export"])?;
            let Subcommand::Export(args) = cli.subcommand else {
                return Err(::anyhow::anyhow!("expected export subcommand"));
            };
            assert_eq!(args.url, None);
            Ok(())
        })
    }

    #[test]
    fn parses_export_subcommand_url_override() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from([
            "shiori",
            "export",
            "--url",
            "http://localhost:3000/base/export",
        ])?;
        let Subcommand::Export(args) = cli.subcommand else {
            return Err(::anyhow::anyhow!("expected export subcommand"));
        };
        assert_eq!(
            args.url,
            Some("http://localhost:3000/base/export".to_string())
        );
        Ok(())
    }

    #[test]
    fn parses_export_subcommand_url_from_env() -> ::anyhow::Result<()> {
        ::temp_env::with_var(
            "SHIORI_EXPORT_URL",
            Some("http://127.0.0.1:3000/custom/export"),
            || {
                let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "export"])?;
                let Subcommand::Export(args) = cli.subcommand else {
                    return Err(::anyhow::anyhow!("expected export subcommand"));
                };
                assert_eq!(
                    args.url,
                    Some("http://127.0.0.1:3000/custom/export".to_string())
                );
                Ok(())
            },
        )
    }

    #[test]
    fn export_flag_overrides_env_url() -> ::anyhow::Result<()> {
        ::temp_env::with_var(
            "SHIORI_EXPORT_URL",
            Some("http://127.0.0.1:3000/from-env/export"),
            || {
                let cli = <Cli as ::clap::Parser>::try_parse_from([
                    "shiori",
                    "export",
                    "--url",
                    "http://127.0.0.1:3000/from-flag/export",
                ])?;
                let Subcommand::Export(args) = cli.subcommand else {
                    return Err(::anyhow::anyhow!("expected export subcommand"));
                };
                assert_eq!(
                    args.url,
                    Some("http://127.0.0.1:3000/from-flag/export".to_string())
                );
                Ok(())
            },
        )
    }

    #[test]
    fn requires_a_subcommand() {
        assert!(<Cli as ::clap::Parser>::try_parse_from(["shiori"]).is_err());
    }
}
