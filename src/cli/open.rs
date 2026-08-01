use std::{env::current_dir, ffi::OsStr, os::unix::ffi::OsStrExt, process::Command};

use anyhow::{Context as _, Result, ensure};

use crate::api::{
    config::{Config, Contest},
    contest::specify_task,
};

#[derive(Debug, clap::Args)]
pub struct Open {
    task: Option<String>,
}

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
            .output()
            .context("failed to open file")?;

        ensure!(
            output.status.success(),
            "failed to open file\nstderr: {}",
            OsStr::from_bytes(&output.stderr).display()
        );

        Ok(())
    }
}
