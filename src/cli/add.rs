use std::{collections::HashSet, env::current_dir};

use crate::api::{
    config::{CONTEST_DATA_FILE_NAME, Config, Contest, Task},
    contest::{get_contest_title, get_tasks_name_and_title, set_task_crate},
    http::{build_client, get_html},
};
use anyhow::{Context, Result};
use reqwest::Url;
use tokio::fs;
use toml_edit::{Array, DocumentMut, Item, Table, Value};

#[derive(Debug, clap::Args)]
pub struct Add {
    contest: String,
}

impl Add {
    pub async fn add(&self) -> Result<()> {
        let (root_dir, config) = Config::read(&current_dir()?)?;
        let client = build_client()?;

        let contest_dir = root_dir.join(&self.contest);
        fs::create_dir_all(&contest_dir).await?;

        let tasks_page_url = Url::parse(&format!(
            "https://atcoder.jp/contests/{}/tasks?lang=ja",
            self.contest
        ))
        .unwrap();
        let tasks_page_html = get_html(&client, tasks_page_url).await?;

        let contest_title = get_contest_title(&tasks_page_html)?;
        let tasks = get_tasks_name_and_title(&tasks_page_html)?;

        let contest_data = Contest {
            name: self.contest.clone(),
            title: contest_title,
            tasks: tasks
                .iter()
                .cloned()
                .map(|(name, title)| Task { name, title })
                .collect(),
        };
        fs::write(
            contest_dir.join(CONTEST_DATA_FILE_NAME),
            &serde_json::to_string_pretty(&contest_data)?,
        )
        .await?;

        let template_file_path = root_dir.join(&config.template.path);
        let template_file_text = fs::read(&template_file_path).await?;

        for (task, _) in &tasks {
            let task_page_url = Url::parse(&format!(
                "https://atcoder.jp/contests/{}/tasks/{task}?lang=ja",
                self.contest
            ))
            .unwrap();
            let task_page_html = get_html(&client, task_page_url)
                .await
                .context(format!("failed to get task `{task}` page"))?;

            set_task_crate(
                &self.contest,
                task,
                &config,
                &contest_dir,
                &template_file_text,
                &task_page_html,
            )
            .await?;
        }

        let cargo_toml_path = root_dir.join("Cargo.toml");

        let workspace_toml = fs::read_to_string(&cargo_toml_path).await?;
        let mut workspace_toml: DocumentMut = workspace_toml.parse()?;

        let workspace_members = workspace_toml
            .entry("workspace")
            .or_insert(Item::Table(Table::new()))
            .as_table_mut()
            .unwrap()
            .entry("members")
            .or_insert(Item::Value(Value::Array(Array::new())))
            .as_array_mut()
            .unwrap();
        let members_set: HashSet<_> = workspace_members
            .iter()
            .map(|member| member.as_str().unwrap().to_string())
            .collect();
        workspace_members.extend(tasks.iter().filter_map(|(task, _)| {
            let task_dir = format!("{}/{}", contest_data.name, task);
            (!members_set.contains(&task_dir)).then_some(task_dir)
        }));

        fs::write(&cargo_toml_path, workspace_toml.to_string()).await?;

        println!("added contest `{}`", contest_data.title);

        Ok(())
    }
}
