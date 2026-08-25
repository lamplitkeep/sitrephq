use serde::Deserialize;
use std::time::Duration;

use crate::source::Source;
use crate::sources::docker::Docker;
use crate::sources::systemd_agent::SystemdAgent;

#[derive(Deserialize)]
pub struct Config {
    pub sources: Vec<SourceConfig>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SourceConfig {
    Docker {
        name: String,
        base: String,
        #[serde(with = "humantime_serde")]
        interval: Duration,
    },
    SystemdAgent {
        name: String,
        url: String,
        #[serde(with = "humantime_serde")]
        interval: Duration,
    },
}

impl SourceConfig {
    pub fn build(self) -> Box<dyn Source> {
        match self {
            SourceConfig::Docker { name, base, interval} => {
                Box::new(Docker::new(name, base, interval))
            }
            SourceConfig::SystemdAgent { name, url, interval} => {
                Box::new(SystemdAgent::new(name, url, interval))
            }
        }
    }
}

pub fn load(path: &str) -> anyhow::Result<Config> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
    Ok(serde_saphyr::from_str(&text)?)
}