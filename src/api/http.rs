use std::{
    fs,
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use anyhow::{Context, Result};
use dirs::home_dir;
use reqwest::{Client, IntoUrl, Url, cookie::Jar};
use scraper::Html;

static SESSION_TOKEN_FILE_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| home_dir().unwrap().join(".actester/session_token"));

pub fn set_session_token(session_token: &str) -> Result<()> {
    let session_token_file_path = &*SESSION_TOKEN_FILE_PATH;
    fs::create_dir_all(session_token_file_path)?;
    fs::write(session_token_file_path, session_token)?;

    Ok(())
}

fn read_session_token() -> Result<Option<String>> {
    let session_token_file_path = &*SESSION_TOKEN_FILE_PATH;
    let session_token = if session_token_file_path.exists() {
        Some(
            fs::read_to_string(session_token_file_path)
                .context("failed to read session_token file")?,
        )
    } else {
        None
    };

    Ok(session_token)
}

pub fn build_client() -> Result<Client> {
    let cookies = Arc::new(Jar::default());

    if let Some(cookie) = read_session_token()? {
        let cookie_str = format!("REVEL_SESSION={cookie}");
        cookies.add_cookie_str(&cookie_str, &Url::parse("https://atcoder.jp")?);
    }

    let client = Client::builder().cookie_provider(cookies).build()?;

    Ok(client)
}

pub async fn get_html(client: &Client, url: impl IntoUrl) -> Result<Html> {
    let response = client.get(url).send().await?;

    let document = response.text().await?;

    let html = Html::parse_document(&document);

    Ok(html)
}
