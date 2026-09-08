use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;
use std::collections::VecDeque;

use crate::source::Source;

const BUCKET_SECS: u64 = 300;
const MAX_BUCKETS: usize = 288;

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

struct Bucket {
    start: u64,
    tx_sum: u64,
    rx_sum: u64,
    n: u32,
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
    history: VecDeque<Bucket>,
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
        Ok(Self { name, base, site, gateway, interval, client, history:VecDeque::new() })
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

    fn record(&mut self, ts: u64, tx: u64, rx: u64) {
        let start = ts - ts % BUCKET_SECS;
        match self.history.back_mut() {
            Some(b) if b.start == start => {
                b.tx_sum += tx;
                b.rx_sum += rx;
                b.n += 1;
            }
            _ => {
                self.history.push_back(Bucket { start, tx_sum: tx, rx_sum: rx, n: 1 });
                while self.history.len() > MAX_BUCKETS {
                    self.history.pop_front();
                }
            }
        }
    }

    fn history_json(&self) -> serde_json::Value {
        self.history
            .iter()
            .map(|b| json!({
                "t": b.start,
                "tx": b.tx_sum / b.n as u64,
                "rx": b.rx_sum / b.n as u64,
            }))
            .collect()
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

        let want = self.gateway.clone();
        let gateway = match want.as_deref() {
            Some(want) => match devices.data.iter().find(|d| &d.name == want) {
                Some(d) => {
                    let s: GatewayStats = self
                        .get(&format!("/sites/{site}/devices/{}/statistics/latest", d.id))
                        .await?;
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    self.record(now, s.uplink.tx_rate_bps, s.uplink.rx_rate_bps);
                    json!({
                        "uptime_sec": s.uptime_sec,
                        "cpu_pct": s.cpu_utilization_pct,
                        "mem_pct": s.memory_utilization_pct,
                        "tx_bps": s.uplink.tx_rate_bps,
                        "rx_bps": s.uplink.rx_rate_bps,
                        "history": self.history_json(),
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


#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> Unifi {
        Unifi::new(
            "t".into(), "httpss://x".into(), Some("s".into()), None,
            "k".into(), true, Duration::from_secs(30),
        ).expect("client builds")
    }

    #[test]
    fn samples_in_same_bucket_average() {
        let mut u = empty();
        u.record(1000, 100, 200); // 1000 -> bucket 900
        u.record(1100, 300, 400); // still bucket 900
        assert_eq!(u.history.len(), 1);
        let b = u.history.back().unwrap();
        assert_eq!(b.start, 900);
        assert_eq!(b.n, 2);
        assert_eq!(b.tx_sum, 400);  // averaged at emit: 200
    }

    #[test]
    fn crossing_boundary_opens_new_bucket() {
        let mut u = empty();
        u.record(1000, 10, 10); // bucket 900
        u.record(1300, 20, 20); // bucket 1200
        assert_eq!(u.history.len(), 2);
    }

    #[test]
    fn history_caps_at_max() {
        let mut u = empty();
        for i in 0..(MAX_BUCKETS as u64 + 50) {
            u.record(i * BUCKET_SECS, 1, 1);
        }
        assert_eq!(u.history.len(), MAX_BUCKETS);
    }

    #[test]
    fn oldest_dropped_first() {
        let mut u = empty();
        for i in 0..(MAX_BUCKETS as u64 + 1) {
            u.record(i * BUCKET_SECS, 1, 1);
        }
        assert_eq!(u.history.front().unwrap().start, BUCKET_SECS);
    }

    #[test]
    fn emit_averages_not_sums() {
        let mut u = empty();
        u.record(0, 100, 100);
        u.record(60, 300, 300); // same bucket 0, avg 200
        let v = u.history_json();
        assert_eq!(v[0]["tx"], 200);
    }
}

