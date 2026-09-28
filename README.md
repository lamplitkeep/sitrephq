# SitRepHQ

A self-hosted status dashboard that doesn't limit you.

<!-- TODO: screenshot from a scrubbed demo config -->
![SitRepHQ](docs/img/sitrep.png)

## Why it exists

Most homelab dashboards are link pages with widgets bolted on. Each widget is a
plugin someone has to write and maintain, and when the service you run isn't on
the list you're out of luck.

SitRepHQ started as a private cockpit for one homelab and was rebuilt so the
structure lives in config instead of code. Two generic source types, `http-json`
and `command`, cover anything with an API or a CLI. You decide which values show
up and what they're called. The dashboard gives you one place to see what is
running, what finished, and what broke, next to the terminals and pages you
already keep open.

It is a single binary with its assets built in. No database, no Node, no
Docker required.

## What it does

- **Status strip.** Every source becomes a zone of state pills: containers,
  systemd units, Pi-hole, UniFi devices, or any value pulled out of JSON.
  Every pill is one of five states: ok, idle, run, ghost, bad.
- **Attention bar.** Anything bad, unknown, or unreachable is pinned to the top
  of the page, from every tab, so one glance is enough.
- **Panes.** Embed web terminals (ttyd running btop, htop, anything), live
  previews of your own sites, and a filterable Pi-hole query log.
- **Tabs.** Split a large setup into focused views, each with its own sources
  and panes. Keys 1 to 9 switch between them.
- **Themes.** Override any color, font, or radius in a small `theme.yml`. Three
  palettes ship in `sitrep/examples/`.

Sources:

| Type | Reads |
| --- | --- |
| `docker` | Containers and health checks, through a read-only socket proxy |
| `systemd-agent` | Units on another host, via `sitrep-agent` |
| `pihole` | Pi-hole v6 stats and the live query stream |
| `unifi` | UniFi Network devices, clients, and a 24h WAN chart |
| `http-json` | Any JSON API, with bearer, header, or basic auth |
| `command` | Any shell command's output, as JSON, a number, or text |

## Install

### Binary (recommended)

Download the latest release, make it executable, and put it on your path:

```sh
curl -LO https://github.com/lamplitkeep/sitrephq/releases/latest/download/sitrep-linux-x86_64
chmod +x sitrep-linux-x86_64
sudo install sitrep-linux-x86_64 /usr/local/bin/sitrep
```

### Docker

Two images are published:

| Tag | Base | Use it when |
| --- | --- | --- |
| `ghcr.io/lamplitkeep/sitrephq:latest` | distroless | You don't use `command` sources. There is no shell in this image. |
| `ghcr.io/lamplitkeep/sitrephq:full` | Debian slim with curl and jq | You use `command` sources. Any other tools they call (the aws CLI, for example) need a derived image. |

Mount the whole config directory, not just `config.yml`, so includes and
`theme.yml` are found:

```sh
docker run -d --name sitrep \
  --network host \
  -v /srv/sitrep:/config:ro \
  --env-file /srv/sitrep/.env \
  ghcr.io/lamplitkeep/sitrephq:latest
```

Host networking matters: inside a container, `127.0.0.1` is the container, not
the host, so sources on the host's loopback won't be reachable otherwise. A
`docker-compose.yml` is in the repo root.

### Cargo

```sh
cargo install sitrephq
```

## Quickstart

```sh
mkdir ~/sitrep && cd ~/sitrep
curl -Lo config.yml https://raw.githubusercontent.com/lamplitkeep/sitrephq/main/sitrep/examples/config.example.yml
curl -Lo .env https://raw.githubusercontent.com/lamplitkeep/sitrephq/main/sitrep/examples/env.example
chmod 600 .env
```

Edit `config.yml`: delete the example sources you don't run and point the rest
at your own hosts. Put the matching secrets in `.env`. Then:

```sh
sitrep check    # validate the config and test your panes
sitrep          # reads ./config.yml, ./.env, and ./theme.yml if present
```

Open http://127.0.0.1:1986.

The example config documents every option inline. For a longer walkthrough see
[docs/configuration.md](docs/configuration.md).

## The agent

`systemd-agent` sources read from `sitrep-agent`, a small binary that runs on
each host you want units from and reports them over HTTP at `/units`.

<!-- TODO: agent release asset name, flags, and unit selection -->
```sh
curl -LO https://github.com/lamplitkeep/sitrephq/releases/latest/download/sitrep-agent-linux-x86_64
chmod +x sitrep-agent-linux-x86_64
sudo install sitrep-agent-linux-x86_64 /usr/local/bin/sitrep-agent
```

Run it under systemd and bind it to the same private network as the dashboard.
It listens on port 9100. Example units for both binaries are in
[docs/deployment.md](docs/deployment.md).

## Security model

SitRepHQ has no login. The network is the perimeter.

- **Bind to a private address.** `bind:` defaults to `127.0.0.1:1986`. To reach
  it from other devices, bind a VPN or tailnet IP. Never bind a public address.
- **Secrets stay out of config.** Config files only name environment variables.
  Values come from the environment or `.env`, and a missing one stops startup.
- **Command sources run shell commands.** Anyone who can edit `config.yml` can
  run code as the sitrep user. Protect it like `.env`.
- **Docker through a proxy.** The `docker` source expects a read-only socket
  proxy. Never expose the raw Docker socket.
- **Panes are loaded by your browser, not proxied.** SitRepHQ sends a
  Content-Security-Policy that allows only the origins in your config, and
  `frame-ancestors 'none'` so the dashboard itself can't be framed.
- **TLS** can be served directly with a `tls:` block, or left to a reverse
  proxy in front.

## Documentation

- [Configuration](docs/configuration.md): every source, pills, includes, and layout
- [Panes](docs/panes.md): terminals, site previews, log panes, and fixing blank panes
- [Deployment](docs/deployment.md): systemd, Docker, TLS, and reverse proxies
- [Presets](docs/presets.md): ready-made source configs for common services
- [Health checks](docs/healthchecks.md): getting accurate container states

## License

MIT. See [LICENSE](LICENSE).

A [Lamplit Keep](https://lamplitkeep.com) project.