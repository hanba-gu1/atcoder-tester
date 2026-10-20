mod api;
mod cli;
mod error;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    cli.subcommand
        .exec()
        .await
        .map_err(|err| match err.downcast::<error::UserError>() {
            Ok(err) => err.into(),
            Err(err) => err,
        })
}
