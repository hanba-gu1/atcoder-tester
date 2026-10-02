use std::env::current_dir;

use anyhow::{Context, Result, anyhow, ensure};

use crate::api::{
    config::{Config, Contest},
    contest::specify_task,
    sample_test::{
        build_for_test, display_all_test_results, get_all_samples, get_sample, sample_test,
        test_all_sample,
    },
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
            build_for_test(&root_dir, &contest_data, task)?;
        }

        if let Some(sample_number) = self.sample {
            let (sample_in, sample_out) = get_sample(&task_dir, sample_number)?
                .with_context(|| anyhow!("Sample {sample_number} doesn't exist."))?;

            sample_test(&contest_dir, &contest_data, task, &sample_in, &sample_out)?;
        } else {
            let samples = get_all_samples(&task_dir)?;
            ensure!(!samples.is_empty(), "No sample testcase was found.");
            let (all_ac, results) = test_all_sample(&contest_dir, &contest_data, task, &samples)?;
            display_all_test_results(&samples, &results)?;
            ensure!(all_ac, "Some sample testcases was not passed.");
        }

        Ok(())
    }
}
