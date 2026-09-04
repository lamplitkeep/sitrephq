use async_trait::async_trait;
use std::time::Duration;

use crate::pihole::PiholeClient;
use crate::source::Source;

pub struct Pihole {
    name: String,
    interval: Duration,
    client: PiholeClient,
}

impl Pihole {
    pub fn new(name: String, base: String, password: String, interval: Duration) -> anyhow::Result<Self> {
        Ok(Self { name, interval, client: PiholeClient::new(base, password)? })
    }
}

#[async_trait]
impl Source for Pihole {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &'static str { "pihole" }
    fn interval(&self) -> Duration { self.interval }

    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value> {
        self.client.get("/api/stats/summary").await
    }
}