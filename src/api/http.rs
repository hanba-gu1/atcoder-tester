use std::{fs, sync::Arc};
#[cfg(target_os = "linux")]
use std::os::unix::fs::PermissionsExt;

use anyhow::{Context, Result};
use dirs::home_dir;
use hooq::hooq;
use reqwest::{Client, IntoUrl, Url, cookie::Jar};
use scraper::Html;

#[hooq(anyhow)]
pub fn set_session_token(session_token: &str) -> Result<()> {
    let actester_dir = home_dir()?.join(".actester");
    let session_token_file_path = actester_dir.join("session_token");
    fs::create_dir_all(&actester_dir)?;
    fs::write(&session_token_file_path, session_token)?;
    #[cfg(target_os = "linux")]
    {
        let mut perm = fs::metadata(&session_token_file_path)?.permissions();
        perm.set_mode(0o600);
        fs::set_permissions(&session_token_file_path, perm)?;
    }

    Ok(())
}

#[hooq(anyhow)]
fn read_session_token() -> Result<Option<String>> {
    let actester_dir = home_dir()?.join(".actester");
    let session_token_file_path = actester_dir.join("session_token");
    let session_token = session_token_file_path
        .exists()
        .then(|| fs::read_to_string(session_token_file_path))
        .transpose()?;

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
