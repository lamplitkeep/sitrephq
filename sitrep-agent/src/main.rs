mod units;

use axum::{routing::get, Json, Router};

async fn units_handler() -> Json<Vec<units::UnitStatus>> {
    let path = std::env::var("AGENT_UNITS")
        .unwrap_or_else(|_| "/etc/sitrep-agent/units".to_string());
    let config = std::fs::read_to_string(&path).unwrap_or_default();
    Json(units::collect(&config))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/units", get(units_handler));
    let bind = std::env::var("AGENT_BIND").unwrap_or_else(|_| "0.0.0.0:9100".to_string());
    let listener = tokio::net::TcpListener::bind(&bind).await.expect("bind failed");
    println!("sitrep-agent on {bind}");
    axum::serve(listener, app).await.expect("serve failed");
}