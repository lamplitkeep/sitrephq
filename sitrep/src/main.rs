mod source;
mod sources;
mod config;
mod pihole;
mod log_stream;
mod assets;
mod check;

use std::collections::HashMap;
use std::sync::Arc;
use axum::{extract::State, extract::Query, routing::get, Json, Router};
use axum::http::{header, HeaderValue};
use axum::response::sse::{Event, Sse};
use tokio::sync::RwLock;
use tower_http::set_header::SetResponseHeaderLayer;
use futures::stream::Stream;

use source::{spawn_poller, Cache, SourceEntry};
use log_stream::{spawn_log_poller, LogTx};

#[derive(Clone)]
struct AppState {
    cache: Cache,
    layout: config::Layout,
    theme: config::Theme,
    label_rules: HashMap<String, config::LabelRules>,
    pill_specs: HashMap<String, Vec<config::PillSpec>>,
    source_order: Vec<String>,
    log_tx: Option<LogTx>,
}

#[derive(serde::Deserialize)]
struct LogFilter {
    client: Option<String>,
    domain: Option<String>,
    status: Option<String>,
}

async fn log_stream(
    State(state): State<AppState>,
    Query(filter): Query<LogFilter>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.log_tx.as_ref().map(|tx| tx.subscribe());
    let stream = async_stream::stream! {
        if let Some(mut rx) = rx {
            while let Ok(row) = rx.recv().await {
                let ok = filter.client.as_deref().map_or(true, |p| log_stream::glob_match(p, &row.client))
                    && filter.domain.as_deref().map_or(true, |p| log_stream::glob_match(p, &row.domain))
                    && filter.status.as_deref().map_or(true, |p| p.eq_ignore_ascii_case(&row.status));
                if ok {
                    if let Ok(json) = serde_json::to_string(&row) {
                        yield Ok(Event::default().data(json));
                    }
                }
            }
        }
    };
    Sse::new(stream)
}

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = std::env::args().collect();
    let is_check = args.iter().any(|a| a == "check");

    let config_path = args.iter()
        .skip(1)
        .find(|a| *a != "check")
        .cloned()
        .unwrap_or_else(|| "config.yml".to_string());

    let config = match config::load(&config_path) {
        Ok(c) => c,
        Err(e) => { eprintln!("config error:\n{e:#}"); std::process::exit(1)}
    };

    if is_check {
        check::run(&config).await;
        return;
    }

    let source_order: Vec<String> = config.sources.iter().map(|s| s.name().to_string()).collect();

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

    let log_tx = config.sources.iter().find_map(|s| match s {
        config::SourceConfig::Pihole { base, password_env, .. } => {
            let pw = std::env::var(password_env).ok()?;
            let client = pihole::PiholeClient::new(base.clone(), pw).ok()?;
            Some(spawn_log_poller(Arc::new(tokio::sync::Mutex::new(client))))
        }
        _ => None,
    });

    let bind = config.bind.clone();
    let tls = config.tls.clone();
    let layout = config.layout.clone().unwrap_or_else(|| config::Layout { panes: vec![], tabs: vec![] });
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
        let mut origins: Vec<String> = layout
            .panes
            .iter()
            .chain(layout.tabs.iter().flat_map(|t| t.panes.iter()))
            .filter(|p| !p.url.is_empty())
            .map(|p| p.url.splitn(4, '/').take(3).collect::<Vec<_>>().join("/"))
            .collect();
        origins.sort();
        origins.dedup();
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

    let state = AppState { cache, layout, theme, label_rules, pill_specs, source_order, log_tx };

    let app = Router::new()
        .route("/api/status", get(status))
        .route("/api/config", get(config_handler))
        .route("/api/queries/stream", get(log_stream))
        .fallback(assets::serve)
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_str(&csp).expect("csp header"),
        ))
        .with_state(state);

    match tls {
        Some(tls) => {
            let cfg = axum_server::tls_rustls::RustlsConfig::from_pem_file(&tls.cert, &tls.key)
                .await
                .expect("loading TLS cert/key");
            let addr: std::net::SocketAddr = bind.parse().expect("invalid bind address");
            println!("listening on https://{bind}");
            axum_server::bind_rustls(addr, cfg)
                .serve(app.into_make_service())
                .await
                .expect("server failed");
        }
        None => {
            let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind failed");
            println!("listening on http://{bind}");
            axum::serve(listener, app).await.expect("server failed");
        }
    }
}

async fn status(State(state): State<AppState>) -> Json<HashMap<String, SourceEntry>> {
    let map = state.cache.read().await.clone();
    Json(map)
}

async fn config_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "theme": state.theme, "layout": state.layout,
        "label_rules": state.label_rules, "pill_specs": state.pill_specs, "source_order": state.source_order, }))
}