mod source;
mod sources;
mod config;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use axum::{extract::State, routing::get, Json, Router};
use tokio::sync::RwLock;

use source::{spawn_poller, Cache, SourceEntry, Source};
use sources::{docker::Docker, systemd_agent::SystemdAgent};

#[tokio::main]
async fn main() {
    let cache: Cache = Arc::new(RwLock::new(HashMap::new()));

    let sources: Vec<Box<dyn Source>> = vec![
        Box::new(Docker::new(
            "docker-host1",
            "http://127.0.0.1:2375",
            Duration::from_secs(30),
        )),
        Box::new(Docker::new(
            "docker-host2",
            "http://100.64.0.11:2375",
            Duration::from_secs(30),
        )),
        Box::new(SystemdAgent::new(
            "systemd-host3",
            "http://100.64.0.12:9100/units",
            Duration::from_secs(30)
        )),
    ];

    for s in sources {
        spawn_poller(s, cache.clone());
    }

    let app = Router::new()
        .route("/api/status", get(status))
        .with_state(cache);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:1986")
        .await
        .expect("bind failed");
    println!("listening on http://127.0.0.1:1986");
    axum::serve(listener, app).await.expect("server failed");
}

async fn status(State(cache): State<Cache>) -> Json<HashMap<String, SourceEntry>> {
    let map = cache.read().await.clone();
    Json(map)
}