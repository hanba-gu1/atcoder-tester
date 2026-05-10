use std::{env::current_dir, path::Path};

use anyhow::{Context as _, Result, ensure};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::Url;
use scraper::{Html, Selector};

use crate::api::{
    config::{Contest, Task},
    http::Requester,
};

pub async fn get_title_and_tasks(
    requester: &Requester,
    contest: &str,
) -> Result<(String, Vec<(String, String)>)> {
    let tasks_url = Url::parse(&format!(
        "https://atcoder.jp/contests/{contest}/tasks?lang=ja"
    ))?;
    let response = requester.get(&tasks_url).await?;
    ensure!(
        response.status().is_success(),
        "failed to get tasks in contest `{contest}` wtih status {}",
        response.status()
    );

    let document = Html::parse_document(&response.text().await?);

    let contest_title_selector = Selector::parse(".contest-title").unwrap();

    let contest_title = document
        .select(&contest_title_selector)
        .next()
        .map(|elem| elem.inner_html().trim().to_string())
        .unwrap_or_else(|| contest.to_string());

    let tr_selector = Selector::parse("tr").unwrap();
    let a_selector = Selector::parse("a").unwrap();

    let mut tasks = Vec::new();

    for tr_elem in document.select(&tr_selector) {
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

    Ok((contest_title, tasks))
}

pub async fn get_samples(
    requester: &Requester,
    contest: &str,
    task: &str,
) -> Result<(Vec<String>, Vec<String>)> {
    let task_url = Url::parse(&format!(
        "https://atcoder.jp/contests/{contest}/tasks/{task}"
    ))?;
    let response = requester.get(&task_url).await?;
    ensure!(
        response.status().is_success(),
        "failed to get samples in task `{task}` in contest `{contest}` wtih status {}",
        response.status()
    );

    let response_text = response.text().await?;
    let document = Html::parse_document(&response_text);

    let mut sample_inputs = Vec::new();
    let mut sample_outputs = Vec::new();

    let section_selector = Selector::parse("section").unwrap();
    let h3_selector = Selector::parse("h3").unwrap();
    let pre_selector = Selector::parse("pre").unwrap();

    for section_elem in document.select(&section_selector) {
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

async fn get_csrf_token(requester: &Requester, contest_name: &str) -> Result<String> {
    let url = Url::parse(&format!(
        "https://atcoder.jp/contests/{contest_name}/submit"
    ))?;
    let response = requester.get(&url).await?;
    ensure!(
        response.status().is_success(),
        "failed to get submit page in contest `{contest_name}` wtih status {}",
        response.status()
    );

    let html = Html::parse_document(&response.text().await?);
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
    requester: &Requester,
    contest_name: &str,
    task_name: &str,
    code: String,
) -> Result<()> {
    let url = Url::parse(&format!(
        "https://atcoder.jp/contests/{contest_name}/submit"
    ))?;
    let csrf_token = get_csrf_token(requester, contest_name).await?;
    let response = requester
        .post(
            &url,
            vec![
                ("data.TaskScreenName".to_string(), task_name.to_string()),
                ("data.LanguageId".to_string(), "6088".to_string()),
                ("sourceCode".to_string(), utf8_percent_encode(&code, NON_ALPHANUMERIC).to_string()),
                ("csrf_token".to_string(), utf8_percent_encode(&csrf_token, NON_ALPHANUMERIC).to_string()),
            ],
        )
        .await?;

    eprintln!("{csrf_token}");

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

    #[tokio::test]
    async fn get_tasks_test() -> Result<()> {
        let requester = Requester::new()?;
        let contest = "abc455";

        let (title, tasks) = get_title_and_tasks(&requester, contest).await?;
        let (tasks, task_titles): (Vec<_>, Vec<_>) = tasks.into_iter().unzip();

        assert_eq!(
            title,
            "Ｓｋｙ株式会社プログラミングコンテスト2026（AtCoder Beginner Contest 455）"
        );
        assert_eq!(
            tasks,
            [
                "abc455_a", "abc455_b", "abc455_c", "abc455_d", "abc455_e", "abc455_f", "abc455_g"
            ]
        );
        assert_eq!(
            task_titles,
            [
                "455",
                "Spiral Galaxy",
                "Vanish",
                "Card Pile Query",
                "Unbalanced ABC Substrings",
                "Merge Slimes 2",
                "Balanced Subarrays"
            ]
        );

        Ok(())
    }

    #[tokio::test]
    async fn get_samples_test() -> Result<()> {
        let requester = Requester::new()?;
        let contest = "abc455";
        let task = "abc455_a";

        let (sample_inputs, sample_outputs) = get_samples(&requester, contest, task).await?;
        assert_eq!(sample_inputs, ["4 5 5\n", "1 3 7\n", "6 6 6\n"]);
        assert_eq!(sample_outputs, ["Yes\n", "No\n", "No\n"]);

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
