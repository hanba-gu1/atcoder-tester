use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

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
            if dir.join("actester.toml").exists() {
                break toml::from_slice(
                    &fs::read(dir.join("actester.toml"))
                        .context("failed to read `actester.toml`")?,
                )?;
            }
            if !dir.pop() {
                bail!(
                    "could not find `actester.toml` in `{}` or any parent directory",
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
            if dir.join("contest.json").exists() {
                let content =
                    fs::read(dir.join("contest.json")).context("failed read `contest.json`")?;
                let contest_data = serde_json::from_slice::<Contest>(&content)?;
                break contest_data;
            }
            if !dir.pop() {
                bail!(
                    "could not find `actester.toml` in `{}` or any parent directory",
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
