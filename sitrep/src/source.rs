use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;


#[async_trait]
pub trait Source: Send {
    fn name(&self) -> &str;
    fn kind(&self) -> &'static str;
    fn interval(&self) -> Duration;
    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value>;
}

#[derive(Serialize, Clone)]
pub struct SourceEntry {
    pub kind: &'static str,
    pub value: Option<serde_json::Value>,
    pub fetched_at: u64,
    pub error: Option<String>,
}


pub type Cache = Arc<RwLock<HashMap<String, SourceEntry>>>;

pub fn spawn_poller(mut source: Box<dyn Source>, cache: Cache) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(source.interval());
        loop {
            ticker.tick().await;
            let name = source.name().to_string();
            let kind = source.kind();
            let entry = match source.fetch().await {
                Ok(value) => SourceEntry {
                    kind,
                    value: Some(value),
                    fetched_at: now(),
                    error: None,
                },
                Err(e) => {
                    eprintln!("[{name}] fetch failed: {e:#}");
                    SourceEntry {
                        kind,
                        value: None,
                        fetched_at: now(),
                        error: Some(e.to_string()),
                    }
                }
            };

            cache.write().await.insert(name, entry);
        }
    });
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
