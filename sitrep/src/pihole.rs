use serde::Deserialize;
use std::time::Duration;
use std::sync::Mutex;

// Every live Pi-hole session in this process, as (base, sid).
// Login adds to it, re-login removes the stale one, shutdown drains it.
static SESSIONS: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());

fn register(base: &str, sid: &str) {
    SESSIONS.lock().unwrap().push((base.to_string(), sid.to_string()));
}

fn unregister(sid: &str) {
    SESSIONS.lock().unwrap().retain(|(_, s)| s != sid);
}

/// Log out every session this process opened. Called once at shutdown.
pub async fn logout_all() {
    let sessions: Vec<(String, String)> = std::mem::take(&mut *SESSIONS.lock().unwrap());
    if sessions.is_empty() {
        return;
    }
    let http = match reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };
    for (base, sid) in sessions {
        let _ = http
            .delete(format!("{base}/api/auth"))
            .header("sid", &sid)
            .send()
            .await;
    }
    eprintln!("[pihole] logged out of open sessions");
}

#[derive(Deserialize)]
struct AuthResponse {
    session: Session,
}

#[derive(Deserialize)]
struct Session {
    valid: bool,
    sid: Option<String>,
}

pub struct PiholeClient {
    base: String,
    password: String,
    sid: Option<String>,
    http: reqwest::Client,
}

impl PiholeClient {
    pub fn new(base: String, password: String) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self { base, password, sid: None, http })
    }

    async fn auth(&mut self) -> anyhow::Result <()> {
        let resp: AuthResponse = self
            .http
            .post(format!("{}/api/auth", self.base))
            .json(&serde_json::json!({ "password": self.password }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        match (resp.session.valid, resp.session.sid) {
            (true, Some(sid)) => {
                register(&self.base, &sid);
                self.sid = Some(sid);
                Ok(())
            }
            _ => anyhow::bail!("pi-hole rejected the app password"),
        }
    }

    pub async fn get(&mut self, path: &str) -> anyhow::Result<serde_json::Value> {
        if self.sid.is_none() {
            self.auth().await?;
        }
        let resp = self.send(path).await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            if let Some(old) = self.sid.take() {
                unregister(&old);
            }
            self.auth().await?;
            let resp = self.send(path).await?;
            return Ok(resp.error_for_status()?.json().await?);
        }
        Ok(resp.error_for_status()?.json().await?)
    }

    pub async fn queries_since(&mut self, _after: u64) -> anyhow::Result<serde_json::Value> {
        self.get("/api/queries?length=100").await
    }

    async fn send(&self, path: &str) -> anyhow::Result<reqwest::Response> {
        let sid = self.sid.as_deref().unwrap_or("");
        Ok(self
            .http
            .get(format!("{}{}", self.base, path))
            .header("sid", sid)
            .send()
            .await?)
    }

}