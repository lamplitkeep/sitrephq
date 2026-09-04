use serde::Deserialize;
use std::time::Duration;

use crate::source::Source;
use crate::sources::docker::Docker;
use crate::sources::systemd_agent::SystemdAgent;
use crate::sources::http_json::HttpJson;
use crate::sources::command::CommandSource;
use crate::sources::pihole::Pihole;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_bind")]
    pub bind: String,
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub layout: Option<Layout>,
}

#[derive(Deserialize, Clone, serde::Serialize)]
pub struct Layout {
    #[serde(default)]
    pub panes: Vec<Pane>,
}

#[derive(Deserialize, Clone, serde::Serialize)]
pub struct Pane {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub kind: PaneKind,
}

#[derive(Deserialize, Clone, Default, serde::Serialize)]
pub struct Theme(pub std::collections::HashMap<String, String>);

fn default_command_timeout() -> Duration {
    Duration::from_secs(30)
}
fn default_bind() -> String { "127.0.0.1:1986".to_string() }

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
    Command {
        name: String,
        run: String,
        #[serde(with = "humantime_serde")]
        interval: Duration,
        #[serde(with = "humantime_serde", default = "default_command_timeout")]
        timeout: Duration,
        #[serde(default)]
        parse: Parse,
    },
    Pihole {
        name: String,
        base: String,
        password_env: String,
        #[serde(with = "humantime_serde")]
        interval: Duration,
    },
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Expect {
    #[default]
    Json,
    Ok,
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Parse {
    #[default]
    Json,
    Number,
    Raw,
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

#[derive(Deserialize, Clone, Copy, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneKind {
    #[default]
    Site,
    Term,
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
            SourceConfig::Command { name, run, interval, timeout, parse } => {
                Ok(Box::new(CommandSource::new(name, run, interval, timeout, parse)))
            }
            SourceConfig::Pihole { name, base, password_env, interval } => {
                let password = std::env::var(&password_env)
                    .map_err(|_| anyhow::anyhow!("environment variable {password_env} not set. Check your .env file"))?;
                Ok(Box::new(Pihole::new(name, base, password, interval)?))
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

impl Config {
    fn validate(&self) -> anyhow::Result<()> {
        if let Some(layout) = &self.layout {
            for pane in &layout.panes {
                let ok = pane.url.starts_with("http://") || pane.url.starts_with("https://");
                if !ok {
                    anyhow::bail!(
                        "pane \"{}\": url must start with http:// or https:// (got {:?})", pane.title, pane.url
                    );
                }
            }
        }
        Ok(())
    }
}

pub fn load_theme(path: &str) -> anyhow::Result<Theme> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(serde_saphyr::from_str(&text)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Theme::default()),
        Err(e) => Err(anyhow::anyhow!("reading {path}: {e}")),
    }
}

pub fn load(path: &str) -> anyhow::Result<Config> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("reading {path}: {e}"))?;
    let config: Config = serde_saphyr::from_str(&text)?;
    config.validate()?;
    Ok(config)
}