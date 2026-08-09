use async_trait::async_trait;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

#[async_trait]
pub trait Source: Send {
    fn name(&self) -> &str;
    fn interval(&self) -> Duration;
    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value>;
}

#[derive(Serialize, Clone)]
pub struct SourceEntry {
    pub value: Option<serde_json::Value>,
    pub fetched_at: u64,
    pub error: Option<String>,
}

pub type Cache = Arc<RwLock<HashMap<String, SourceEntry>>>;
