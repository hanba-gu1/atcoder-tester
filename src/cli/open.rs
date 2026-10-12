use std::{env::current_dir, process::Command};

use anyhow::Result;
use hooq::hooq;

use crate::{
    api::{
        config::{Config, Contest},
        contest::specify_task,
    },
    ensure,
};

#[derive(Debug, clap::Args)]
pub struct Open {
    task: Option<String>,
}

#[hooq(anyhow)]
impl Open {
    pub fn open(&self) -> Result<()> {
        let current_dir = current_dir()?;
        let (_, config) = Config::read(&current_dir)?;
        let (contest_dir, contest_data) = Contest::read(&current_dir)?;

        let task = specify_task(&contest_dir, &contest_data, self.task.as_deref())?;
        let task_dir = contest_dir.join(&task.name);

        let command = config
            .open
            .command
            .replace("$file", &task_dir.join("src/main.rs").to_string_lossy());
        let command: Vec<_> = command.split_ascii_whitespace().collect();

        let output = Command::new(command[0])
            .args(if command.is_empty() {
                &[][..]
            } else {
                &command[1..]
            })
            .output()?;

        ensure!(
            output.status.success(),
            "failed to open file\nstderr: {}",
            String::from_utf8_lossy(&output.stderr),
        );

        Ok(())
    }
}
