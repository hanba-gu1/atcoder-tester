use std::{env::current_dir, thread::sleep, time::Duration};

use anyhow::{Result, bail};
use arboard::Clipboard;

use crate::api::{
    config::{Config, Contest},
    contest::specify_task,
    expand_files::expand_files,
    sample_test::{build_for_test, display_all_test_results, get_all_samples, test_all_sample},
};

#[derive(Debug, clap::Args)]
pub struct Clip {
    task: Option<String>,
    #[arg(long = "no-test")]
    no_test: bool,
    #[arg(long = "no-build")]
    no_build: bool,
}

impl Clip {
    pub fn clip(&self) -> Result<()> {
        let mut clipboard = Clipboard::new()?;

        let current_dir = current_dir()?;
        let (root_dir, config) = Config::read(&current_dir)?;
        let (contest_dir, contest_data) = Contest::read(&current_dir)?;

        let task = specify_task(&contest_dir, &contest_data, self.task.as_deref())?;

        let task_dir = contest_dir.join(&task.name);

        if !self.no_test && config.clip.sample_test {
            let samples = get_all_samples(&task_dir)?;

            if !samples.is_empty() {
                if !self.no_build {
                    build_for_test(&root_dir, &contest_data, task)?;
                }
                let (all_ac, results) =
                    test_all_sample(&contest_dir, &contest_data, task, &samples)?;
                display_all_test_results(&samples, &results);
                if !all_ac {
                    clipboard.set_text("")?;
                    bail!("Some sample testcases wasn't passed.");
                }
            } else {
                eprintln!("No sample testcase was found.");
            }
        }

        let result_file = expand_files(
            &task_dir.join("src/main.rs"),
            &root_dir.join(&config.libs.path),
        )?;
        clipboard.set_text(&result_file)?;
        eprintln!("Clip!");

        sleep(Duration::from_millis(200));

        Ok(())
    }
}
