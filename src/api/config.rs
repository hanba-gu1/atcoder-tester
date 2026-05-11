use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub const CONFIG_FILE_NAME: &str = "actester.toml";
pub const CONTEST_DATA_FILE_NAME: &str = "contest.json";

#[derive(Debug, Deserialize)]
pub struct Config {
    pub template: Template,
    pub libs: Libs,
    pub generate: Generate,
    pub submit: Submit,
    pub clip: Clip,
}

impl Config {
    pub fn read(dir: &Path) -> Result<(PathBuf, Self)> {
        let mut dir = dir.to_path_buf();

        let config = loop {
            let config_file_path = dir.join(CONFIG_FILE_NAME);
            if config_file_path.exists() {
                break toml::from_slice(
                    &fs::read(config_file_path)
                        .context(format!("failed to read `{CONFIG_FILE_NAME}`"))?,
                )?;
            }
            if !dir.pop() {
                bail!(
                    "could not find `{CONFIG_FILE_NAME}` in `{}` or any parent directory",
                    dir.display()
                );
            }
        };

        Ok((dir, config))
    }
}

#[derive(Debug, Deserialize)]
pub struct Template {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Libs {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct Generate {
    pub cargo_toml: String,
}

#[derive(Debug, Deserialize)]
pub struct Submit {
    pub sample_test: bool,
}

#[derive(Debug, Deserialize)]
pub struct Clip {
    pub sample_test: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Contest {
    pub name: String,
    pub title: String,
    pub tasks: Vec<Task>,
}

impl Contest {
    pub fn read(dir: &Path) -> Result<(PathBuf, Self)> {
        let mut dir = dir.to_path_buf();

        let contest_data = loop {
            let contest_data_file_path = dir.join(CONTEST_DATA_FILE_NAME);
            if contest_data_file_path.exists() {
                let content = fs::read(contest_data_file_path)
                    .context(format!("failed read `{CONTEST_DATA_FILE_NAME}`"))?;
                let contest_data = serde_json::from_slice::<Contest>(&content)?;
                break contest_data;
            }
            if !dir.pop() {
                bail!(
                    "could not find `{CONTEST_DATA_FILE_NAME}` in `{}` or any parent directory",
                    dir.display()
                );
            }
        };

        Ok((dir, contest_data))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub title: String,
}
