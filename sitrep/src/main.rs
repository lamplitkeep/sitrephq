mod source;
mod sources;
mod config;
mod pihole;

use std::collections::HashMap;
use std::sync::Arc;
use axum::{extract::State, routing::get, Json, Router};
use axum::http::{header, HeaderValue};
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

use source::{spawn_poller, Cache, SourceEntry};

#[derive(Clone)]
struct AppState {
    cache: Cache,
    layout: config::Layout,
    theme: config::Theme,
    label_rules: HashMap<String, config::LabelRules>,
    pill_specs: HashMap<String, Vec<config::PillSpec>>,
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();

    let config_path = std::env::args().nth(1).unwrap_or_else(|| "config.yml".to_string());
    let config = match config::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("config error:\n{e:#}");
            std::process::exit(1);
        }
    };

    let label_rules: std::collections::HashMap<String, config::LabelRules> = config
        .sources
        .iter()
        .filter_map(|s| match s {
            config::SourceConfig::SystemdAgent { name, label_rules, .. } =>
                Some((name.clone(), label_rules.clone())),
            _ => None,
        })
        .collect();

    let pill_specs: std::collections::HashMap<String, Vec<config::PillSpec>> = config
        .sources
        .iter()
        .filter_map(|s| match s {
            config::SourceConfig::HttpJson { name, pills, .. } if !pills.is_empty() => 
                Some((name.clone(), pills.clone())),
            config::SourceConfig::Command { name, pills, .. } if !pills.is_empty() =>
                Some((name.clone(), pills.clone())),
            _ => None,
        })
        .collect();

    let bind = config.bind.clone();
    let layout = config.layout.clone().unwrap_or_else(|| config::Layout { panes: vec![] });
    let theme_path = std::path::Path::new(&config_path)
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("theme.yml");
    let theme = match config::load_theme(theme_path.to_str().unwrap_or("theme.yml")) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("theme error:\n{e:#}");
            std::process::exit(1);
        }
    };


    let csp = {
        let origins: Vec<String> = layout
            .panes
            .iter()
            .map(|p| p.url.splitn(4, '/').take(3).collect::<Vec<_>>().join("/"))
            .collect();
        let frame_src = if origins.is_empty() { "'none'".to_string() } else { origins.join(" ") };
        format!("frame-ancestors 'none'; frame-src {frame_src}")
    };

    let cache: Cache = Arc::new(RwLock::new(HashMap::new()));

    for sc in config.sources {
        match sc.build() {
            Ok(s) => spawn_poller(s, cache.clone()),
            Err(e) => {
                eprintln!("source config error:\n{e:#}");
                std::process::exit(1)
            }
        }
    }

    let state = AppState { cache, layout, theme, label_rules, pill_specs };

    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/config", get(config_handler))
        .fallback_service(ServeDir::new("static"))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_str(&csp).expect("csp header"),
        ))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .expect("bind failed");
    println!("listening on http://{bind}");
    axum::serve(listener, app).await.expect("server failed");
}

async fn status(State(state): State<AppState>) -> Json<HashMap<String, SourceEntry>> {
    let map = state.cache.read().await.clone();
    Json(map)
}

async fn config_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "theme": state.theme, "layout": state.layout, "label_rules": state.label_rules, "pill_specs": state.pill_specs, }))
}