mod oidc;
mod token_store;

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
    Login,
}

#[::tokio::main]
async fn main() -> ::anyhow::Result<()> {
    match <Cli as ::clap::Parser>::parse().subcommand {
        Subcommand::Export => ::anyhow::bail!("export is not yet implemented"),
        Subcommand::Login => ::anyhow::bail!("login is not yet implemented"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_login_subcommand() -> ::anyhow::Result<()> {
        let cli = <Cli as ::clap::Parser>::try_parse_from(["shiori", "login"])?;
        assert!(matches!(cli.subcommand, Subcommand::Login));
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
