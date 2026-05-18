use std::{
    borrow::Cow,
    env::{self, current_dir},
    path::{Path, PathBuf},
    process::Command,
};

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

        let terminal: Cow<Path> = if let Some(terminal) = config.open.terminal.as_deref() {
            terminal.into()
        } else {
            PathBuf::from(env::var("SHELL")?).into()
        };

        let command = config
            .open
            .command
            .replace("$file", &task_dir.join("src/main.rs").to_string_lossy());

        let output = Command::new(terminal.as_ref())
            .arg(&command)
            .output()
            .context("failed to open file")?;

        ensure!(output.status.success(), "failed to open file");

        Ok(())
    }
}
