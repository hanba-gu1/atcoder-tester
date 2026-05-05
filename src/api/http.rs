use std::{fs, sync::Arc, time::Duration};

use anyhow::Result;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::{Client, Response, Url, cookie::Jar};
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

fn build_client() -> Result<Client> {
    let cookies = Arc::new(Jar::default());
    if let Ok(cookie) = fs::read_to_string("~/.actester/session_cookie") {
        let cookie_str = format!("REVEL_SESSION={cookie}");
        cookies.add_cookie_str(&cookie_str, &Url::parse("https://atcoder.jp")?);
    }

    let client = Client::builder().cookie_provider(cookies).build()?;

    Ok(client)
}

enum Request {
    Get(Url),
    Post {
        url: Url,
        body: Vec<(String, String)>,
    },
}

#[derive(Debug)]
pub struct Requester {
    handle: JoinHandle<Result<()>>,
    sender: mpsc::Sender<(oneshot::Sender<Result<Response>>, Request)>,
}

impl Requester {
    pub fn new() -> Result<Self> {
        let client = build_client()?;

        let (sender, mut receiver) = mpsc::channel::<(oneshot::Sender<_>, _)>(256);

        let handle = tokio::spawn(async move {
            loop {
                let Some((once_sender, request)) = receiver.recv().await else {
                    continue;
                };
                match request {
                    Request::Get(url) => {
                        let response = client.get(url).send().await.map_err(|err| err.into());
                        once_sender.send(response).unwrap();
                    }
                    Request::Post { url, body } => {
                        let body = body
                            .into_iter()
                            .map(|(key, value)| {
                                format!("{}={}", key, utf8_percent_encode(&value, NON_ALPHANUMERIC))
                            })
                            .collect::<Vec<_>>()
                            .join("&");
                        let responce = client
                            .post(url)
                            .body(body)
                            .send()
                            .await
                            .map_err(|err| err.into());
                        once_sender.send(responce).unwrap();
                    }
                }

                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });

        Ok(Self { handle, sender })
    }

    async fn send_message(&self, request: Request) -> Result<Response> {
        let (once_sender, once_receiver) = oneshot::channel();
        self.sender.send((once_sender, request)).await?;
        once_receiver.await?
    }

    pub async fn get(&self, url: &Url) -> Result<Response> {
        self.send_message(Request::Get(url.clone())).await
    }

    pub async fn post(&self, url: &Url, body: Vec<(String, String)>) -> Result<Response> {
        self.send_message(Request::Post {
            url: url.clone(),
            body,
        })
        .await
    }
}

impl Drop for Requester {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
