use serde::Deserialize;
use std::time::Duration;

use crate::source::Source;
use crate::sources::docker::Docker;
use crate::sources::systemd_agent::SystemdAgent;
use crate::sources::http_json::HttpJson;

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
    HttpJson {
        name: String,
        url: String,
        #[serde(with = "humantime_serde")]
        interval: Duration,
        #[serde(default)]
        auth: Option<AuthConfig>,
        #[serde(default)]
        insecure: bool,
        #[serde(default)]
        expect: Expect,
    },
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    #[default]
    Json,
    Ok,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthConfig {
    BearerEnv(String),
    Header { name: String, value_env: String },
    Basic { user: String, password_env: String },
}

pub enum ResolvedAuth {
    Bearer(String),
    Header { name: String, value: String },
    Basic { user: String, password: String },
}

impl SourceConfig {
    pub fn build(self) -> anyhow::Result<Box<dyn Source>> {
        match self {
            SourceConfig::Docker { name, base, interval} => {
                Ok(Box::new(Docker::new(name, base, interval)))
            }
            SourceConfig::SystemdAgent { name, url, interval} => {
                Ok(Box::new(SystemdAgent::new(name, url, interval)))
            }
            SourceConfig::HttpJson { name, url, interval, auth, insecure, expect }  => {
                let auth = auth.map(AuthConfig::resolve).transpose()?;
                Ok(Box::new(HttpJson::new(name, url, interval, auth, insecure, expect)?))
            }
        }
    }
}

impl AuthConfig {
    fn resolve(self) -> anyhow::Result<ResolvedAuth> {
        let get = |var: &str| {
            std::env::var(var)
                .map_err(|_| anyhow::anyhow!("environment variable {var} not set (Check your .env file)"))
        };
        Ok(match self {
            AuthConfig::BearerEnv(var) => ResolvedAuth::Bearer(get(&var)?),
            AuthConfig::Header { name, value_env } => ResolvedAuth::Header { name, value: get(&value_env)? },
            AuthConfig::Basic { user, password_env } => ResolvedAuth::Basic { user, password: get(&password_env)? },

        })
    }
}

pub fn load(path: &str) -> anyhow::Result<Config> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
    Ok(serde_saphyr::from_str(&text)?)
}