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
    Login,
    Init(Init),
    Add(Add),
    #[clap(alias = "t")]
    Test(Test),
    #[clap(alias = "s")]
    Submit(Submit),
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
            Submit(submit) => todo!(),
            Clip(clip) => clip.clip()?,
        }

        Ok(())
    }
}
