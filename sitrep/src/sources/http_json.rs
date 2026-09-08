use async_trait::async_trait;
use std::time::Duration;

use crate::config::{Expect, ResolvedAuth};
use crate::source::Source;

pub struct HttpJson {
    name: String,
    url: String,
    interval: Duration,
    expect: Expect,
    client: reqwest::Client,
}

impl HttpJson {
    pub fn new(
        name: String,
        url: String,
        interval: Duration,
        auth: Option<ResolvedAuth>,
        insecure: bool,
        expect: Expect,
    ) -> anyhow::Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        match auth {
            Some(ResolvedAuth::Bearer(token)) => {
                let mut v: reqwest::header::HeaderValue = format!("Bearer {token}").parse()?;
                v.set_sensitive(true);
                headers.insert(reqwest::header::AUTHORIZATION, v);
            }
            Some(ResolvedAuth::Header { name, value }) => {
                let n: reqwest::header::HeaderName = name.parse()?;
                let mut v: reqwest::header::HeaderValue = value.parse()?;
                v.set_sensitive(true);
                headers.insert(n, v);
            }
            Some(ResolvedAuth::Basic { user, password }) => {
                use base64::Engine;
                let cred = base64::engine::general_purpose::STANDARD
                    .encode(format!("{user}:{password}"));
                let mut v: reqwest::header::HeaderValue = format!("Basic {cred}").parse()?;
                v.set_sensitive(true);
                headers.insert(reqwest::header::AUTHORIZATION, v);
            }
            None => {}
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .danger_accept_invalid_certs(insecure)
            .build()?;

        Ok(Self { name, url, interval, expect, client })
    }
}

#[async_trait]
impl Source for HttpJson {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &'static str {
        "generic"
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value> {
        let resp = self.client.get(&self.url).send().await?.error_for_status()?;
        match self.expect {
            Expect::Json => Ok(resp.json().await?),
            Expect::Ok => Ok(serde_json::json!({ "up": true })),
        }

    }
}