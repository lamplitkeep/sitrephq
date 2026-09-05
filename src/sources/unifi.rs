use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use crate::source::Source;

#[derive(Deserialize)]
struct Envelope<T> {
    #[serde(rename = "totalCount")]
    total_count: u64,
    data: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Site {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Device {
    id: String,
    name: String,
    model: String,
    state: String,
    firmware_updatable: bool,
}

#[derive(Deserialize)]
struct Ignored {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatewayStats {
    uptime_sec: u64,
    cpu_utilization_pct: f64,
    memory_utilization_pct: f64,
    uplink: Uplink,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Uplink {
    tx_rate_bps: u64,
    rx_rate_bps: u64,
}

pub struct Unifi {
    name: String,
    base: String,
    site: Option<String>,
    gateway: Option<String>,
    interval: Duration,
    client: reqwest::Client,
}

impl Unifi {
    pub fn new(
        name: String,
        base: String,
        site: Option<String>,
        gateway: Option<String>,
        key: String,
        insecure: bool,
        interval: Duration,
    ) -> anyhow::Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        let mut v: reqwest::header::HeaderValue = key.parse()?;
        v.set_sensitive(true);
        headers.insert("X-API-KEY", v);
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .default_headers(headers)
            .danger_accept_invalid_certs(insecure)
            .build()?;
        Ok(Self { name, base, site, gateway, interval, client })
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let url = format!("{}/proxy/network/integration/v1{}", self.base, path);
        Ok(self.client.get(url).send().await?.error_for_status()?.json().await?)
    }

    async fn site_id(&mut self) -> anyhow::Result<String> {
        if let Some(s) = &self.site {
            return Ok(s.clone());
        }
        let sites: Envelope<Site> = self.get("/sites").await?;
        let id = sites
            .data
            .first()
            .map(|s| s.id.clone())
            .ok_or_else(|| anyhow::anyhow!("controller reports no sites"))?;
        self.site = Some(id.clone());
        Ok(id)
    }
}

#[async_trait]
impl Source for Unifi {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> &'static str { "unifi" }
    fn interval(&self) -> Duration { self.interval }

    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value> {
        let site = self.site_id().await?;

        let devices: Envelope<Device> = self.get(&format!("/sites/{site}/devices")).await?;
        let clients: Envelope<Ignored> = self.get(&format!("/sites/{site}/clients?limit=1")).await?;

        let gateway = match &self.gateway {
            Some(want) => match devices.data.iter().find(|d| &d.name == want) {
                Some(d) => {
                    let s: GatewayStats = self
                        .get(&format!("/sites/{site}/devices/{}/statistics/latest", d.id))
                        .await?;
                    json!({
                        "uptime_sec": s.uptime_sec,
                        "cpu_pct": s.cpu_utilization_pct,
                        "mem_pct": s.memory_utilization_pct,
                        "tx_bps": s.uplink.tx_rate_bps,
                        "rx_bps": s.uplink.rx_rate_bps,
                    })
                }
                None => json!(null),
            },
            None => json!(null),
        };

        let devices: Vec<serde_json::Value> = devices
            .data
            .iter()
            .map(|d| json!({
                "name": d.name,
                "model": d.model,
                "state": d.state,
                "firmware_updatable": d.firmware_updatable,
            }))
            .collect();

        Ok(json!({
            "devices": devices,
            "clients": clients.total_count,
            "gateway": gateway,
        }))
    }
}


