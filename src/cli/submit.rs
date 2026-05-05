use std::{
    env::current_dir,
    fs,
    process::{self, Stdio},
};

use anyhow::{Context as _, Result, ensure};

use crate::api::{
    config::{Config, Contest}, contest::{specify_task, submit_code}, expand_files::expand_files, http::Requester, sample_test::sample_test
};

#[derive(Debug, clap::Args)]
pub struct Submit {
    task: Option<String>,
    #[arg(long = "no-test")]
    no_test: bool,
    #[arg(long = "no-build")]
    no_build: bool,
}

impl Submit {
    pub async fn submit(&self) -> Result<()> {
        let requester = Requester::new()?;
        let current_dir = current_dir()?;
        let (root_dir, config) = Config::read(&current_dir)?;
        let (contest_dir, contest_data) = Contest::read(&current_dir)?;

        let task = specify_task(&contest_dir, &contest_data, self.task.as_deref())?;

        let task_dir = contest_dir.join(&task.name);

        let mut all_ac = true;

        if !self.no_test && config.submit.sample_test {
            if !self.no_build {
                let build_output = process::Command::new("cargo")
                    .args([
                        "build",
                        "--package",
                        &format!("{}-{}", contest_data.name, task.name),
                    ])
                    .current_dir(&root_dir)
                    .stderr(Stdio::inherit())
                    .output()
                    .context("failed to build")?;
                ensure!(build_output.status.success(), "falied to build");
            }

            for i in 1.. {
                let in_file = task_dir.join(format!("samples/{i}.in"));
                let out_file = task_dir.join(format!("samples/{i}.out"));
                if !in_file.exists() {
                    break;
                }

                let sample_in = fs::read_to_string(&in_file)?;
                let sample_out = fs::read_to_string(&out_file)?;

                all_ac &= sample_test(
                    &contest_dir,
                    &contest_data,
                    task,
                    i,
                    &sample_in,
                    &sample_out,
                )?;
            }
        }

        if all_ac {
            let code = expand_files(&task_dir.join("src/main.rs"), &root_dir.join(&config.libs.path))?;
            submit_code(&requester, &contest_data.name, &task.name, code).await?;
            eprintln!("Submit!");
        }

        Ok(())
    }
}
