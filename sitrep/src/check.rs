use crate::config::{Config, Layout, Pane, PaneKind};

pub async fn run(config: &Config) {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client");

    let dash_origin = format!("http://{}", config.bind);

    let panes = collect_panes(&config.layout);
    if panes.is_empty() {
        println!("no panes configured.");
        return;
    }

    println!("checking {} pane(s)...\n", panes.len());
    for pane in panes {
        if matches!(pane.kind, PaneKind::Log) { continue; }
        check_pane(&client, &pane, &dash_origin).await;
    }
}

fn collect_panes(layout: &Option<Layout>) -> Vec<Pane> {
    let Some(l) = layout else { return vec![] };
    let mut v = l.panes.clone();
    for t in &l.tabs {
        v.extend(t.panes.clone());
    }
    v
}

async fn check_pane(client: &reqwest::Client, pane: &Pane, dash_origin: &str) {
    println!("  {} ({}) ... ", pane.title, pane.url);

    let resp = match client.get(&pane.url).send().await {
        Ok(r) => r,
        Err(e) => {
            println!("UNREACHABLE from server -- {}", e);
            println!("      (advisory: the viewing browsers path may differ - VPN, DNS, cert trust)");
            return;
        }
    };

    let headers = resp.headers();
    let mut problems = Vec::new();

    if dash_origin.starts_with("https://") && pane.url.starts_with("http://") {
        problems.push("MIXED CONTENT: an HTTPS dashboard cannot frame an HTTP page\n
                                    Serve this target over HTTPS".to_string());
    }

    if let Some(xfo) = headers.get("x-frame-options") {
        let v = xfo.to_str().unwrap_or("");
        problems.push(format!("X-FRAME-OPTIONS: {v:?} - blocks framing\n
        XFO has no allowlist; remove it and use a scoped CSP on the target instead:\n
        Content-Security-Policy: frame-ancestors 'self' {dash_origin}"));
    }

    if let Some(csp) = headers.get("content-security-policy") {
        let v = csp.to_str().unwrap_or("");
        if v.contains("frame-ancestors") && !v.contains(dash_origin) && !v.contains('*') {
            problems.push(format!("CSP frame-ancestors doesn't include the dashboard origin\n\
            Add it (exact origin, including port)\n\
            frame-ancestors ... {dash_origin}"));
        }
    }

    if problems.is_empty() {
        println!("OK")
    } else {
        println!("BLOCKED");
        for p in problems {
            println!("           -> {p}");
        }
    }
}