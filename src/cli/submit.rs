use std::env::current_dir;

use anyhow::{Result, ensure};
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::api::{
    config::{Config, Contest},
    contest::{specify_task, submit_code},
    expand_files::expand_files,
    http::build_client,
    sample_test::{build_for_test, display_all_test_results, get_all_samples, test_all_sample},
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
        let client = build_client()?;
        let current_dir = current_dir()?;
        let (root_dir, config) = Config::read(&current_dir)?;
        let (contest_dir, contest_data) = Contest::read(&current_dir)?;

        let task = specify_task(&contest_dir, &contest_data, self.task.as_deref())?;

        let task_dir = contest_dir.join(&task.name);

        if !self.no_test && config.submit.sample_test {
            let samples = get_all_samples(&task_dir)?;

            if samples.is_empty() {
                let proceed = Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt("No sample testcase was found. Do you want to submit?")
                    .default(true)
                    .interact()?;
                ensure!(proceed, "Canceled submit.");
            } else {
                if !self.no_build {
                    build_for_test(&root_dir, &contest_data, task)?;
                }
                let (all_ac, results) =
                    test_all_sample(&contest_dir, &contest_data, task, &samples)?;
                display_all_test_results(&samples, &results)?;
                ensure!(all_ac, "Some sample testcases was not passed.");
            }
        }

        let code = expand_files(
            &task_dir.join("src/main.rs"),
            &root_dir.join(&config.libs.path),
        )?;
        submit_code(&client, &contest_data.name, &task.name, code).await?;
        eprintln!("Submit!");

        Ok(())
    }
}
