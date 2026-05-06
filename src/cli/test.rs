use std::{
    env::current_dir,
    fs,
    process::{self, Stdio},
};

use anyhow::{Context as _, Result, ensure};

use crate::api::{
    config::{Config, Contest},
    contest::specify_task,
    sample_test::sample_test,
};

#[derive(Debug, clap::Args)]
pub struct Test {
    task: Option<String>,
    #[arg(short = 's', long = "sample")]
    sample: Option<usize>,
    #[arg(long = "no-build")]
    no_build: bool,
}

impl Test {
    pub fn test(&self) -> Result<()> {
        let current_dir = current_dir()?;

        let (root_dir, _) = Config::read(&current_dir)?;
        let (contest_dir, contest_data) = Contest::read(&current_dir)?;
        let task = specify_task(&contest_dir, &contest_data, self.task.as_deref())?;

        let task_dir = contest_dir.join(&task.name);

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

        if let Some(sample_number) = self.sample {
            let in_file = task_dir.join(format!("samples/{sample_number}.in"));
            let out_file = task_dir.join(format!("samples/{sample_number}.out"));
            ensure!(in_file.exists(), "sample doesn't exist");

            let sample_in = fs::read_to_string(&in_file)?;
            let sample_out = fs::read_to_string(&out_file)?;

            sample_test(
                &contest_dir,
                &contest_data,
                task,
                sample_number,
                &sample_in,
                &sample_out,
            )?;
        } else {
            for i in 1.. {
                let in_file = task_dir.join(format!("samples/{i}.in"));
                let out_file = task_dir.join(format!("samples/{i}.out"));
                if !in_file.exists() {
                    break;
                }

                let sample_in = fs::read_to_string(&in_file)?;
                let sample_out = fs::read_to_string(&out_file)?;

                sample_test(
                    &contest_dir,
                    &contest_data,
                    task,
                    i,
                    &sample_in,
                    &sample_out,
                )?;
            }
        }

        Ok(())
    }
}
