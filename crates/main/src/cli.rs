#[derive(::clap::Parser)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(::clap::Subcommand)]
pub(crate) enum Subcommand {
    GenerateSecret,
    Serve(ServeArgs),
}

#[derive(::clap::Args)]
pub(crate) struct ServeArgs {
    #[arg(default_value = "", env = "BASE_PATH", long)]
    pub base_path: String,
    #[arg(env = "COOKIE_SIGNING_SECRET", long)]
    pub cookie_signing_secret: String,
    #[arg(env = "DATABASE_ID", long)]
    pub database_id: String,
    #[arg(env = "OIDC_CLIENT_ID", long)]
    pub oidc_client_id: String,
    #[arg(env = "OIDC_CLIENT_SECRET", long)]
    pub oidc_client_secret: String,
    #[arg(env = "OIDC_ISSUER_URL", long)]
    pub oidc_issuer_url: String,
    #[arg(env = "OIDC_REDIRECT_URI", long)]
    pub oidc_redirect_uri: String,
    #[arg(default_value_t = 3000, env = "PORT", long)]
    pub port: u16,
    #[arg(env = "PROJECT_ID", long)]
    pub project_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_vars() -> [(&'static str, Option<&'static str>); 9] {
        [
            ("BASE_PATH", Some("/app")),
            (
                "COOKIE_SIGNING_SECRET",
                Some("test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding"),
            ),
            ("DATABASE_ID", Some("test_database_id")),
            ("OIDC_CLIENT_ID", Some("test_client_id")),
            ("OIDC_CLIENT_SECRET", Some("test_client_secret")),
            ("OIDC_ISSUER_URL", Some("https://issuer.example.com")),
            (
                "OIDC_REDIRECT_URI",
                Some("http://localhost:3000/auth/callback"),
            ),
            ("PORT", Some("8080")),
            ("PROJECT_ID", Some("test_project_id")),
        ]
    }

    #[test]
    fn serve_reads_all_variables() -> ::anyhow::Result<()> {
        ::temp_env::with_vars(all_vars(), || {
            let cli = <Cli as ::clap::Parser>::try_parse_from(["app", "serve"])?;
            let Subcommand::Serve(args) = cli.subcommand else {
                return Err(::anyhow::anyhow!("expected serve subcommand"));
            };
            assert_eq!(args.base_path, "/app");
            assert_eq!(
                args.cookie_signing_secret,
                "test_cookie_signing_secret_that_is_at_least_64_bytes_long_padding"
            );
            assert_eq!(args.database_id, "test_database_id");
            assert_eq!(args.oidc_client_id, "test_client_id");
            assert_eq!(args.oidc_client_secret, "test_client_secret");
            assert_eq!(args.oidc_issuer_url, "https://issuer.example.com");
            assert_eq!(
                args.oidc_redirect_uri,
                "http://localhost:3000/auth/callback"
            );
            assert_eq!(args.port, 8080_u16);
            assert_eq!(args.project_id, "test_project_id");
            Ok(())
        })
    }

    #[test]
    fn serve_port_defaults_to_3000() -> ::anyhow::Result<()> {
        ::temp_env::with_vars(
            all_vars()
                .into_iter()
                .map(|(k, v)| if k == "PORT" { (k, None) } else { (k, v) })
                .collect::<Vec<_>>(),
            || {
                let cli = <Cli as ::clap::Parser>::try_parse_from(["app", "serve"])?;
                let Subcommand::Serve(args) = cli.subcommand else {
                    return Err(::anyhow::anyhow!("expected serve subcommand"));
                };
                assert_eq!(args.port, 3000_u16);
                Ok(())
            },
        )
    }

    #[test]
    fn serve_base_path_defaults_to_empty() -> ::anyhow::Result<()> {
        ::temp_env::with_vars(
            all_vars()
                .into_iter()
                .map(|(k, v)| if k == "BASE_PATH" { (k, None) } else { (k, v) })
                .collect::<Vec<_>>(),
            || {
                let cli = <Cli as ::clap::Parser>::try_parse_from(["app", "serve"])?;
                let Subcommand::Serve(args) = cli.subcommand else {
                    return Err(::anyhow::anyhow!("expected serve subcommand"));
                };
                assert_eq!(args.base_path, "");
                Ok(())
            },
        )
    }

    #[test]
    fn serve_fails_when_required_variable_is_missing() {
        ::temp_env::with_vars(
            all_vars()
                .into_iter()
                .map(|(k, v)| {
                    if k == "COOKIE_SIGNING_SECRET" {
                        (k, None)
                    } else {
                        (k, v)
                    }
                })
                .collect::<Vec<_>>(),
            || {
                assert!(<Cli as ::clap::Parser>::try_parse_from(["app", "serve"]).is_err());
            },
        );
    }
}
