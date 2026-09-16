use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use crate::pihole::PiholeClient;

#[derive(Clone, serde::Serialize)]
pub struct LogRow {
    pub id: u64,
    pub time: f64,
    pub domain: String,
    pub client: String,
    pub status: String,
    pub qtype: String,
}

pub type LogTx = broadcast::Sender<LogRow>;

pub fn spawn_log_poller(client: Arc<Mutex<PiholeClient>>) -> LogTx {
    let (tx, _rx) = broadcast::channel::<LogRow>(500);
    let out = tx.clone();
    tokio::spawn(async move {
        let mut watermark: u64 = 0;
        let mut backoff = 2u64;
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(backoff)).await;

            let fetched = {
                let mut c = client.lock().await;
                c.queries_since(watermark).await
            };
            let val = match fetched {
                Ok(v) => { backoff = 2; v }
                Err(e) => {
                    eprintln!("[pihole-log] {e:#} (retrying in {}s", backoff.min(60));
                    backoff = (backoff * 2).min(60);
                    continue;
                }
            };
            let Some(rows) = val.get("queries").and_then(|q| q.as_array()) else { continue };
            let mut new_rows: Vec<LogRow> = rows.iter().filter_map(|q| {
                let id = q.get("id")?.as_u64()?;
                if id <= watermark { return None; }
                Some(LogRow {
                    id,
                    time: q.get("time")?.as_f64().unwrap_or(0.0),
                    domain: q.get("domain")?.as_str().unwrap_or("").to_string(),
                    client: q.get("client")?.get("ip")?.as_str().unwrap_or("").to_string(),
                    status: q.get("status")?.as_str().unwrap_or("").to_string(),
                    qtype: q.get("type")?.as_str().unwrap_or("").to_string(),
                })
            }).collect();
            new_rows.sort_by_key(|r| r.id);
            for row in new_rows {
                watermark = watermark.max(row.id);
                let _ = out.send(row);
            }
        }
    });
    tx
}

pub fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" || pattern.is_empty() { return true; }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return value.starts_with(prefix);
    }
    pattern == value
}

#[cfg(test)]
mod tests {
    use super::glob_match;
    #[test]
    fn matching() {
        assert!(glob_match("192.168.70.*", "192.168.70.124"));
        assert!(!glob_match("192.168.70.*", "192.168.7.124"));
        assert!(glob_match("*", "anything"));
        assert!(glob_match("exact.com", "exact.com"));
        assert!(!glob_match("exact.com", "other.com"));
        assert!(glob_match("", "anything"));
    }
}