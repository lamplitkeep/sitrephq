use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;

use crate::config::Parse;
use crate::source::Source;

pub struct CommandSource {
    name: String,
    run: String,
    interval: Duration,
    timeout: Duration,
    parse: Parse,
}

impl CommandSource {
    pub fn new(name: String, run: String, interval: Duration, timeout: Duration, parse: Parse) -> Self {
        Self { name, run, interval, timeout, parse }
    }
}

#[async_trait]
impl Source for CommandSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> &'static str {
        "generic"
    }

    fn interval(&self) -> Duration {
        self.interval
    }

    async fn fetch(&mut self) -> anyhow::Result<serde_json::Value> {
        let child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&self.run)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let output = tokio::time::timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_| anyhow::anyhow!("command timed out after {:?}", self.timeout))??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stderr = stderr.trim();
            if stderr.is_empty() {
                anyhow::bail!("command failed ({})", output.status);
            }
            anyhow::bail!("command failed ({}): {}", output.status, stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        match self.parse {
            Parse::Json => Ok(serde_json::from_str(stdout.trim())?),
            Parse::Number => {
                let n: f64 = stdout.trim().parse()
                    .map_err(|_| anyhow::anyhow!("stdout is not a number: {:?}", stdout.trim()))?;
                Ok(json!({ "value": n }))
            }
            Parse::Raw => Ok(json!({ "output": stdout.trim() })),

        }
    }
}