mod source;
mod sources;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use source::{spawn_poller, Cache, Source};
use sources::docker::Docker;

#[tokio::main]
async fn main() {
    let cache: Cache = Arc::new(RwLock::new(HashMap::new()));

    let docker = Docker::new(
        "docker-host1",
        "http://127.0.0.1:2375",
        Duration::from_secs(30),
    );

    spawn_poller(Box::new(docker), cache.clone());

    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let map = cache.read().await;
        for (name, entry) in map.iter() {
            let summary = match (&entry.value, &entry.error) {
                (Some(v), _) => format!(
                    "{} containers",
                    v.as_array().map(|a| a.len()).unwrap_or(0)
                ),
                (None, Some(e)) => format!("ERROR: {e}"),
                (None, None) => "empty".to_string(),
            };
            println!("[{}] fetched at={} {}", name, entry.fetched_at, summary);
        }
    }
}