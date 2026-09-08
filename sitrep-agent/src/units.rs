use serde::Serialize;
use std::process::Command;

#[derive(Serialize)]
pub struct UnitStatus {
    pub name: String,
    pub group: String,
    pub active: String,
    pub sub: String,
    #[serde(rename = "type")]
    pub unit_type: String,
    pub result: String,
    pub since: String,
    pub last_exit: String,
    pub restarts: u32,
}

fn field<'a>(lines: &'a [(&str, &str)], key: &str) -> &'a str {
    lines.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).unwrap_or("")
}

pub fn query(name: &str, group: &str) -> UnitStatus {
    let out = Command::new("systemctl")
        .args([
            "show", name, "-p",
            "ActiveState,SubState,Type,Result,NRestarts,ExecMainStartTimestamp,ExecMainExitTimestamp",
        ])
        .output();

    let unknown = || UnitStatus {
        name: name.to_string(),
        group: group.to_string(),
        active: "unknown".to_string(),
        sub: String::new(),
        unit_type: String::new(),
        result: String::new(),
        since: String::new(),
        last_exit: String::new(),
        restarts: 0,
    };

    let out = match out {
        Ok(o) => o,
        Err(_) => return unknown(),
    };

    let text = String::from_utf8_lossy(&out.stdout);
    let pairs: Vec<(&str, &str)> = text.lines().filter_map(|l| l.split_once('=')).collect();

    UnitStatus {
        name: name.to_string(),
        group: group.to_string(),
        active: field(&pairs, "ActiveState").to_string(),
        sub: field(&pairs, "SubState").to_string(),
        unit_type: field(&pairs, "Type").to_string(),
        result: field(&pairs, "Result").to_string(),
        since: field(&pairs, "ExecMainStartTimestamp").to_string(),
        last_exit: field(&pairs, "ExecMainExitTimestamp").to_string(),
        restarts: field(&pairs, "NRestarts").parse().unwrap_or(0),
    }
}

pub fn members(target: &str) -> Vec<String> {
    let out = Command::new("systemctl")
        .args(["list-dependencies", "--plain", target])
        .output();
    let out = match out {
        Ok(o) => o,
        Err(_) => return vec![],
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .skip(1)
        .map(|l| l.trim())
        .filter(|l| l.ends_with(".service"))
        .map(String::from)
        .collect()
}

pub fn collect(config: &str) -> Vec<UnitStatus> {
    let mut out = Vec::new();
    for name in config.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        if name.ends_with(".target") {
            out.push(query(name, ""));
            for m in members(name) {
                out.push(query(&m, name));
            }
        } else {
            out.push(query(name, ""));
        }
    }
    out
}