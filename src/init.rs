use crate::{conductor, config};
use anyhow::{Context, Result};
use clap::Parser;
use tokio::net::TcpListener;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct CLIArgs {
    #[clap(short, long)]
    pub config: String,
}

pub async fn main() -> Result<()> {
    let cli_args = CLIArgs::parse();

    let config = std::fs::read_to_string(&cli_args.config)
        .with_context(|| format!("Failed to read config from {}", cli_args.config))?;

    let config: config::Config = toml::from_str(&config).context("Config is invalid")?;

    let invoker_server = TcpListener::bind(config.listen.invokers.clone())
        .await
        .with_context(|| {
            format!(
                "Failed to listen on {:?} (this address is from field listen.invokers of the \
                 configuration file)",
                config.listen.invokers
            )
        })?;

    let conductor = Box::leak(Box::new(conductor::Conductor {}));

    loop {
        let (socket, _addr) = invoker_server.accept().await?;
        tokio::spawn(conductor.accept_invoker_connection(socket));
    }
}
