use async_trait::async_trait;
use std::time::Duration;

use crate::source::Source;

pub struct SystemdAgent {
    name: String,
    url: String,
    interval: Duration,
    client: reqwest::Client,
}

impl SystemdAgent {
    pub fn new(name: impl Into<String>, url: impl Into<String>, interval: Duration) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            interval,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("reqwest client construction is foolproof"),
        }
    }
}

#[async_trait]
impl Source for SystemdAgent {
    fn name(&self) -> &str {
        &self.name
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value> {
        Ok(self
            .client
            .get(&self.url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}