use std::{collections::HashSet, env::current_dir, iter};

use crate::api::{
    config::{Config, Contest, Task},
    contest::{get_samples, get_title_and_tasks},
    http::Requester,
};
use anyhow::Result;
use futures::future::try_join_all;
use tokio::fs;
use toml_edit::{Array, DocumentMut, Item, Table, Value};

#[derive(Debug, clap::Args)]
pub struct Add {
    contest: String,
}

impl Add {
    pub async fn add(&self) -> Result<()> {
        let (root_dir, config) = Config::read(&current_dir()?)?;
        let requester = Requester::new()?;

        let contest_dir = root_dir.join(&self.contest);
        fs::create_dir_all(&contest_dir).await?;

        let (contest_title, tasks) = get_title_and_tasks(&requester, &self.contest).await?;

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
            contest_dir.join("contest.json"),
            &serde_json::to_string_pretty(&contest_data)?,
        )
        .await?;

        let add_tasks = tasks.iter().map(|(task, _)| {
            let root_dir = &root_dir;
            let config = &config;
            let requester = &requester;
            let contest_dir = &contest_dir;
            let task_dir = contest_dir.join(task);
            let task_crate_name = format!("{}-{}", self.contest, task);
            let new_cargo_toml = config
                .generate
                .cargo_toml
                .replace("$name", &task_crate_name);
            let samples_dir = task_dir.join("samples");

            async move {
                fs::create_dir_all(&task_dir).await?;
                fs::write(task_dir.join("Cargo.toml"), &new_cargo_toml).await?;

                fs::create_dir_all(task_dir.join("src")).await?;
                fs::copy(
                    root_dir.join(&config.template.path),
                    task_dir.join("src/main.rs"),
                )
                .await?;

                fs::create_dir_all(&samples_dir).await?;
                let (sample_inputs, sample_outputs) =
                    get_samples(requester, &self.contest, task).await?;

                let create_sample_files = iter::zip(&sample_inputs, &sample_outputs)
                    .enumerate()
                    .map(|(i, (sample_in, sample_out))| {
                        let samples_dir = &samples_dir;
                        async move {
                            fs::write(samples_dir.join(format!("{}.in", i + 1)), sample_in).await?;
                            fs::write(samples_dir.join(format!("{}.out", i + 1)), sample_out)
                                .await?;
                            Result::<()>::Ok(())
                        }
                    });

                try_join_all(create_sample_files).await?;

                eprintln!("added task `{task}`");

                Result::<()>::Ok(())
            }
        });

        try_join_all(add_tasks).await?;

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
