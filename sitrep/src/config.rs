use serde::Deserialize;
use std::time::Duration;

use crate::source::Source;
use crate::sources::docker::Docker;
use crate::sources::systemd_agent::SystemdAgent;
use crate::sources::http_json::HttpJson;
use crate::sources::command::CommandSource;
use crate::sources::pihole::Pihole;
use crate::sources::unifi::Unifi;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default = "default_bind")]
    pub bind: String,
    pub sources: Vec<SourceConfig>,
    #[serde(default)]
    pub layout: Option<Layout>,
}

#[derive(Deserialize, Clone, Default, serde::Serialize)]
pub struct LabelRules {
    #[serde(default)]
    pub strip_prefix: Option<String>,
    #[serde(default)]
    pub strip_suffix: Option<String>,
    #[serde(default)]
    pub aliases: std::collections::HashMap<String, String>,
}

#[derive(Deserialize, Clone, Default, serde::Serialize)]
pub struct PillSpec {
    pub path: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub map: std::collections::HashMap<String, String>,
}

#[derive(Deserialize, Clone, serde::Serialize)]
pub struct Layout {
    #[serde(default)]
    pub panes: Vec<Pane>,   // flat - untabbed
    #[serde(default)]
    pub tabs: Vec<Tab>,     // grouped - tabbed
}

#[derive(Deserialize, Clone, serde::Serialize)]
pub struct Pane {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub kind: PaneKind,
}

#[derive(Deserialize, Clone, serde::Serialize)]
pub struct Tab {
    #[serde(default)]
    pub name: Option<String>,  // No name -> phonetic default (ALPHA, BRAVO..)
    #[serde(default)]
    pub sources: Vec<String>,  // empty = show ALL sources
    #[serde(default)]
    pub panes: Vec<Pane>,
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
        #[serde(default)]
        label_rules: LabelRules,
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
        #[serde(default)]
        pills: Vec<PillSpec>,
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
        #[serde(default)]
        pills: Vec<PillSpec>,
    },
    Pihole {
        name: String,
        base: String,
        password_env: String,
        #[serde(with = "humantime_serde")]
        interval: Duration,
    },
    Unifi {
        name: String,
        base: String,
        #[serde(default)]
        site: Option<String>,
        #[serde(default)]
        gateway: Option<String>,
        key_env: String,
        #[serde(default)]
        insecure: bool,
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
    Header { name: String, #[serde(default)] value: Option<String>, value_env: String },
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
            SourceConfig::Docker { name, base, interval } => {
                Ok(Box::new(Docker::new(name, base, interval)))
            }
            SourceConfig::SystemdAgent { name, url, interval, label_rules: _ } => {
                Ok(Box::new(SystemdAgent::new(name, url, interval)))
            }
            SourceConfig::HttpJson { name, url, interval, auth, insecure, expect, pills: _ } => {
                let auth = auth.map(AuthConfig::resolve).transpose()?;
                Ok(Box::new(HttpJson::new(name, url, interval, auth, insecure, expect)?))
            }
            SourceConfig::Command { name, run, interval, timeout, parse, pills: _ } => {
                Ok(Box::new(CommandSource::new(name, run, interval, timeout, parse)))
            }
            SourceConfig::Pihole { name, base, password_env, interval } => {
                let password = std::env::var(&password_env)
                    .map_err(|_| anyhow::anyhow!("environment variable {password_env} not set. Check your .env file"))?;
                Ok(Box::new(Pihole::new(name, base, password, interval)?))
            }
            SourceConfig::Unifi { name, base, site, gateway, key_env, insecure, interval } => {
                let key = std::env::var(&key_env)
                    .map_err(|_| anyhow::anyhow!("environment variable {key_env} not set. Check your .env file"))?;
                Ok(Box::new(Unifi::new(name, base, site, gateway, key, insecure, interval)?))
            }
        }
    }
    pub fn name(&self) -> &str {
        match self {
            SourceConfig::Docker { name, .. }
            | SourceConfig::SystemdAgent { name, .. }
            | SourceConfig::HttpJson { name, .. }
            | SourceConfig::Command { name, .. }
            | SourceConfig::Pihole { name, .. }
            | SourceConfig::Unifi { name, .. } => name,
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
            AuthConfig::Header { name, value, value_env } => {
                let secret = get(&value_env)?;
                let v = match value {
                    Some(tpl) => tpl.replace("{}", &secret),
                    None => secret,
                };
                ResolvedAuth::Header { name, value: v }
            }
            AuthConfig::Basic { user, password_env } => ResolvedAuth::Basic { user, password: get(&password_env)? },

        })
    }
}

impl Config {
    fn validate(&self) -> anyhow::Result<()> {
        let Some(layout) = &self.layout else { return Ok(()) };

        let check_url = |title: &str, url: &str| -> anyhow::Result<()> {
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                anyhow::bail!("pane \"{title}\": url must start with http:// or https:// (got {url:?})");
            }
            Ok(())
        };

        for pane in &layout.panes {
            check_url(&pane.title, &pane.url)?;
        }

        let known: Vec<&str> = self.sources.iter().map(|s| s.name()).collect();
        for (i, tab) in layout.tabs.iter().enumerate() {
            let label = tab.name.clone().unwrap_or_else(|| format!("tab {}", i + 1));
            for pane in &tab.panes {
                check_url(&pane.title, &pane.url)?;
            }
            for s in &tab.sources {
                if !known.contains(&s.as_str()) {
                    anyhow::bail!(
                        "{label}: sources lists {s:?}, but no source with that name is declared (known: {})",
                        known.join(", ")
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

    let root = {
        let p = std::path::Path::new(path).parent().unwrap_or_else(|| std::path::Path::new(""));
        if p.as_os_str().is_empty() { std::path::PathBuf::from(".") } else { p.to_path_buf() }
    };

    let options = serde_saphyr::options! {}.with_include_resolver(
        move |req: serde_saphyr::IncludeRequest| -> Result<serde_saphyr::ResolvedInclude, serde_saphyr::IncludeResolveError> {
            let spec = req.spec;

            //confinement checks
            if spec.starts_with('/') {
                return Err(serde_saphyr::IncludeResolveError::Message(
                    format!("include {spec:?}: absolute paths not allowed")));
            }
            if !(spec.ends_with(".yaml") || spec.ends_with(".yml")) {
                return Err(serde_saphyr::IncludeResolveError::Message(
                    format!("include {spec:?}: must be a .yaml/.yml file")));
            }

            let canon_root = root.canonicalize()?;                       // io::Error -> Message via From, then ?
            let canon = root.join(spec).canonicalize()
                .map_err(|_| serde_saphyr::IncludeResolveError::Message(
                    format!("include {spec:?}: not found")))?;
            if !canon.starts_with(&canon_root) {
                return Err(serde_saphyr::IncludeResolveError::Message(
                    format!("include {spec:?}: resolves outside the config directory")));
            }

            let body = std::fs::read_to_string(&canon)?;                // io::Error -> Message via From
            Ok(serde_saphyr::ResolvedInclude::new(
                spec.to_string(),
                spec.to_string(),
                serde_saphyr::InputSource::from_string(body),
            ))
        },
    );

    let config: Config = serde_saphyr::from_str_with_options(&text, options)?;
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Config {
        serde_saphyr::from_str(yaml).expect("should parse")
    }

    #[test]
    fn docker_source_parses() {
        let c = parse("
sources:
  - type: docker
    name: d
    base: http://x:2375
    interval: 30s
");
        assert_eq!(c.sources.len(), 1);
        match &c.sources[0] {
            SourceConfig::Docker { name, base, interval } => {
                assert_eq!(name, "d");
                assert_eq!(base, "http://x:2375");
                assert_eq!(*interval, Duration::from_secs(30));
            }
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn humantime_intervals() {
        let c = parse("
sources:
  - type: docker
    name: d
    base: http://x:2375
    interval: 6h
");
        match &c.sources[0] {
            SourceConfig::Docker { interval, .. } => {
                assert_eq!(*interval, Duration::from_secs(6 * 3600));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn command_defaults_timeout() {
        let c = parse("
sources:
  - type: command
    name: c
    run: echo hi
    interval: 1m
");
        match &c.sources[0] {
            SourceConfig::Command { timeout, parse, .. } => {
                assert_eq!(*timeout, Duration::from_secs(30));
                assert!(matches!(parse, Parse::Json));
            }
            _ => panic!()
        }
    }

    #[test]
    fn unknown_type_is_error() {
        let r: Result<Config, _> = serde_saphyr::from_str("
sources:
  - type: gibberish
    name: x
");
        assert!(r.is_err());
    }

    #[test]
    fn missing_required_field_is_error() {
        let r: Result<Config, _> = serde_saphyr::from_str("
sources:
  - type: docker
    name: d
    interval: 30s
");
        assert!(r.is_err());
    }

    #[test]
    fn bind_defaults_when_absent() {
        let c = parse("
sources: []
");
        assert_eq!(c.bind, "127.0.0.1:1986");
    }

    #[test]
    fn validate_rejects_javascript_url() {
        let c = parse("
sources: []
layout:
  panes:
    - title: x
      url: \"javascript:alert(1)\"
");
        assert!(c.validate().is_err());
    }

    #[test]
    fn validate_accepts_https_pane() {
        let c = parse("
sources: []
layout:
  panes:
    - title: x
      url: https://example.com
");
        assert!(c.validate().is_ok());
    }

    #[test]
    fn tab_unknown_source_is_error() {
        let c = parse("
sources:
  - type: docker
    name: d
    base: http://x:2375
    interval: 30s
layout:
  tabs:
    - name: X
      sources: [d, nope]
");
        assert!(c.validate().is_err());
    }

    #[test]
    fn tab_known_sources_pass() {
        let c = parse("
sources:
  - type: docker
    name: d
    base: http://x:2375
    interval: 30s
layout:
  tabs:
    - sources: [d]
");
        assert!(c.validate().is_ok());
    }

    #[test]
    fn include_cannot_escape_root() {
        // an include pointing outside the config dir must fail, not read the file
        let dir = std::env::temp_dir().join("sitrep_test_inc");
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config.yml");
        std::fs::write(&cfg, "bind: 127.0.0.1:1986\nsources:\n  - !include ../../../etc/passwd.yaml\n").unwrap();
        let r = load(cfg.to_str().unwrap());
        assert!(r.is_err(), "root-escaping include must be refused");
    }
}



