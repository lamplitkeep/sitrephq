use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use crate::source::Source;

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Container {
    id: String,
    names: Vec<String>,
    state: String,
    status: String,
    health: Option<Health>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Health {
    status: String,
}

pub struct Docker {
    name: String,
    base: String,
    interval: Duration,
    client: reqwest::Client,
}

impl Docker {
    pub fn new(name: impl Into<String>, base: impl Into<String>, interval: Duration) -> Self {
        Self {
            name: name.into(),
            base: base.into(),
            interval,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("reqwest client construction isn't the issue")
        }
    }
}

#[async_trait]
impl Source for Docker {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &'static str {
        "docker"
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/containers/json?all=true", self.base);
        let containers: Vec<Container> = self
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let out: Vec<serde_json::Value> = containers
            .iter()
            .map(|c| {
                json!({
                    "name": c.names.first().map(|n| n.trim_start_matches('/')).unwrap_or(""),
                    "state": c.state,
                    "status": c.status,
                    "health": c.health.as_ref().map(|h| h.status.as_str()),
                    "id": c.id
                })
            })
            .collect();
        Ok(serde_json::Value::Array(out))
    }
}

