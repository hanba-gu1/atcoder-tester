use std::{env::current_dir, iter, path::Path, time::Duration};

use anyhow::{Context as _, Result, ensure};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use tokio::{fs, join};

use crate::api::{
    config::{Config, Contest, Task},
    http::get_html,
};

pub fn get_contest_title(html: &Html) -> Result<String> {
    let contest_title_selector = Selector::parse(".contest-title").unwrap();

    let contest_title = html
        .select(&contest_title_selector)
        .next()
        .map(|elem| elem.inner_html().trim().to_string())
        .context("failed to get contest title")?;

    Ok(contest_title)
}

pub fn get_tasks_name_and_title(html: &Html) -> Result<Vec<(String, String)>> {
    let tr_selector = Selector::parse("tr").unwrap();
    let a_selector = Selector::parse("a").unwrap();

    let mut tasks = Vec::new();

    for tr_elem in html.select(&tr_selector) {
        if let Some(a_elem) = tr_elem.select(&a_selector).nth(1)
            && let Some(href) = a_elem.attr("href")
        {
            let task_name = href
                .trim_end_matches('/')
                .split('/')
                .next_back()
                .unwrap()
                .to_string();
            let task_title = a_elem.inner_html();
            tasks.push((task_name, task_title));
        }
    }

    Ok(tasks)
}

pub async fn get_samples(html: &Html) -> Result<(Vec<String>, Vec<String>)> {
    let mut sample_inputs = Vec::new();
    let mut sample_outputs = Vec::new();

    let section_selector = Selector::parse("section").unwrap();
    let h3_selector = Selector::parse("h3").unwrap();
    let pre_selector = Selector::parse("pre").unwrap();

    for section_elem in html.select(&section_selector) {
        let Some(h3_elem) = section_elem.select(&h3_selector).next() else {
            continue;
        };
        let h3_text = h3_elem.inner_html();
        let h3_text = h3_text.trim();
        if h3_text.starts_with("入力例") {
            let Some(pre_elem) = section_elem.select(&pre_selector).next() else {
                continue;
            };
            let pre_text = pre_elem.inner_html();
            sample_inputs.push(pre_text);
        } else if h3_text.starts_with("出力例") {
            let Some(pre_elem) = section_elem.select(&pre_selector).next() else {
                continue;
            };
            let pre_text = pre_elem.inner_html();
            sample_outputs.push(pre_text);
        }
    }

    Ok((sample_inputs, sample_outputs))
}

fn parse_selected_task(task: &str) -> Result<usize> {
    let task = task
        .parse()
        .ok()
        .or_else(|| parse_task_num_from_enletter(task))
        .context("task must be specified by number or alphabet")?;

    Ok(task)
}

fn parse_task_num_from_enletter(task: &str) -> Option<usize> {
    let task = task.trim().to_ascii_lowercase();
    if !task.chars().all(|c| c.is_ascii_lowercase()) {
        None?
    }

    let mut ret = task.chars().next()? as usize - 97;

    for c in task.chars().skip(1) {
        let c = c as usize - 97;
        ret = (ret + 1) * 26 + c;
    }

    Some(ret + 1)
}

pub fn specify_task<'a>(
    contest_dir: &Path,
    contest_data: &'a Contest,
    task_number: Option<&str>,
) -> Result<&'a Task> {
    let current_dir = current_dir()?;

    let ret = if let Some(task_num) = task_number {
        let task_num = parse_selected_task(task_num)?;
        ensure!(task_num > 0, "task number must be positive");
        contest_data
            .tasks
            .get(task_num - 1)
            .context("task doesn't exist")?
    } else {
        ensure!(current_dir != contest_dir, "task must be specified");
        let mut temp = current_dir.clone();

        while temp.parent().unwrap() != contest_dir {
            temp.pop();
        }

        let task_name = temp.file_name().unwrap().to_string_lossy().into_owned();
        contest_data
            .tasks
            .iter()
            .find(|task| task.name == task_name)
            .context("task doesn't exist")?
    };
    Ok(ret)
}

pub async fn set_task_crate(
    contest_name: &str,
    task_name: &str,
    config: &Config,
    contest_dir: &Path,
    template_file_text: &[u8],
    task_page_html: &Html,
) -> Result<()> {
    let task_dir = contest_dir.join(task_name);
    let task_crate_name = format!("{}-{}", contest_name, task_name);
    let new_cargo_toml = config
        .generate
        .cargo_toml
        .replace("$name", &task_crate_name);
    let samples_dir = task_dir.join("samples");

    fs::create_dir_all(&task_dir).await?;
    fs::write(task_dir.join("Cargo.toml"), &new_cargo_toml).await?;

    fs::create_dir_all(task_dir.join("src")).await?;
    fs::write(task_dir.join("src/main.rs"), &template_file_text).await?;

    let create_files = async {
        let (sample_inputs, sample_outputs) = get_samples(task_page_html).await?;

        fs::create_dir_all(&samples_dir).await?;
        for (i, (sample_in, sample_out)) in iter::zip(&sample_inputs, &sample_outputs).enumerate() {
            let samples_dir = &samples_dir;
            fs::write(samples_dir.join(format!("{}.in", i + 1)), sample_in).await?;
            fs::write(samples_dir.join(format!("{}.out", i + 1)), sample_out).await?;
        }

        eprintln!("added task `{task_name}`");
        Result::<()>::Ok(())
    };

    join!(create_files, tokio::time::sleep(Duration::from_millis(400))).0?;

    Ok(())
}

async fn get_csrf_token(html: &Html) -> Result<String> {
    let csrf_selector = Selector::parse("input[name=csrf_token]").unwrap();

    let csrf_token = html
        .select(&csrf_selector)
        .next()
        .context("could not find csrf_token")?
        .attr("value")
        .unwrap()
        .to_string();

    Ok(csrf_token)
}

pub async fn submit_code(
    client: &Client,
    contest_name: &str,
    task_name: &str,
    source_code: String,
) -> Result<()> {
    let submit_page_url = Url::parse(&format!(
        "https://atcoder.jp/contests/{contest_name}/submit"
    ))?;

    let submit_page_html = get_html(client, submit_page_url.clone())
        .await
        .context("failed to get submit page")?;

    let rust_language_id = "6088";
    let encoded_source_code = utf8_percent_encode(&source_code, NON_ALPHANUMERIC).to_string();
    let csrf_token = get_csrf_token(&submit_page_html).await?;
    let encoded_csrf_token = utf8_percent_encode(&csrf_token, NON_ALPHANUMERIC).to_string();

    let request_body = [
        ("data.TaskScreenName", task_name),
        ("data.LanguageId", rust_language_id),
        ("sourceCode", &encoded_source_code),
        ("csrf_token", &encoded_csrf_token),
    ];

    let request_body = request_body
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");

    let response = client
        .post(submit_page_url)
        .body(request_body)
        .send()
        .await?;

    ensure!(
        response.status().is_redirection(),
        "failed to submit with status {}",
        response.status()
    );

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::api::http::build_client;

    #[tokio::test]
    async fn get_contest_title_test() -> Result<()> {
        let client = build_client()?;
        let tasks_url = Url::parse("https://atcoder.jp/contests/abc457/tasks?lang=ja").unwrap();
        let html = get_html(&client, tasks_url).await?;

        let contest_title = get_contest_title(&html)?;
        assert_eq!(
            contest_title,
            "Polaris.AI プログラミングコンテスト 2026（AtCoder Beginner Contest 457）"
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_tasks_name_and_title_test() -> Result<()> {
        let client = build_client()?;
        let tasks_url = Url::parse("https://atcoder.jp/contests/abc457/tasks?lang=ja").unwrap();
        let html = get_html(&client, tasks_url).await?;

        let tasks = get_tasks_name_and_title(&html)?;
        let (names, titles): (Vec<_>, Vec<_>) = tasks.into_iter().unzip();

        assert_eq!(
            names,
            [
                "abc457_a", "abc457_b", "abc457_c", "abc457_d", "abc457_e", "abc457_f", "abc457_g"
            ]
        );
        assert_eq!(
            titles,
            [
                "Array",
                "Arrays",
                "Long Sequence",
                "Raise Minimum",
                "Crossing Table Cloth",
                "Second Gap",
                "Catch All Apples"
            ]
        );

        Ok(())
    }

    #[test]
    fn parse_task_num_from_enletter_test() {
        assert_eq!(parse_task_num_from_enletter("A"), Some(1));
        assert_eq!(parse_task_num_from_enletter("b"), Some(2));
        assert_eq!(parse_task_num_from_enletter("z"), Some(26));
        assert_eq!(parse_task_num_from_enletter("Aa"), Some(27));
        assert_eq!(parse_task_num_from_enletter("aB"), Some(28));
        assert_eq!(parse_task_num_from_enletter("AZ"), Some(52));
        assert_eq!(parse_task_num_from_enletter("ba"), Some(53));
        assert_eq!(parse_task_num_from_enletter("zy"), Some(701));
        assert_eq!(parse_task_num_from_enletter("zz"), Some(702));
        assert_eq!(parse_task_num_from_enletter("aaa"), Some(703));

        assert!(parse_task_num_from_enletter("12").is_none());
        assert!(parse_task_num_from_enletter("abc.").is_none());
    }
}
