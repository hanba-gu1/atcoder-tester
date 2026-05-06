mod add;
mod clip;
mod init;
mod login;
mod submit;
mod test;

use add::Add;
use anyhow::Result;
use init::Init;
use login::login;
use submit::Submit;
use test::Test;

use crate::cli::clip::Clip;

#[derive(Debug, clap::Parser)]
pub struct Cli {
    #[clap(subcommand)]
    pub subcommand: SubCommands,
}

#[derive(Debug, clap::Subcommand)]
pub enum SubCommands {
    /// enter your session token to login
    Login,
    /// initialize directory
    Init(Init),
    /// add contest and download sample test case
    Add(Add),
    /// test sample case
    #[clap(alias = "t")]
    Test(Test),
    /// submit code
    #[clap(alias = "s")]
    Submit(Submit),
    /// copy in clipboard
    #[clap(alias = "c")]
    Clip(Clip),
}

impl SubCommands {
    pub async fn exec(self) -> Result<()> {
        use SubCommands::*;

        match self {
            Login => login()?,
            Init(init) => init.init()?,
            Add(add) => add.add().await?,
            Test(test) => test.test()?,
            Submit(submit) => submit.submit().await?,
            Clip(clip) => clip.clip()?,
        }

        Ok(())
    }
}
