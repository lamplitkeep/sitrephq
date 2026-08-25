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
    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.yml".to_string());
    let config = match config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("config error:\n{e:#}");
            std::process::exit(1);
        }
    };

    let cache: Cache = Arc::new(RwLock::new(HashMap::new()));

    for sc in config.sources {
        spawn_poller(sc.build(), cache.clone());
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