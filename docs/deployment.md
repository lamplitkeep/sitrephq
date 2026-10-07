# Deployment

SitRepHQ is one binary with its web assets built in. The recommended setup is
that binary running under systemd on a machine in your private network. Docker
works too.

Wherever it runs, remember that SitRepHQ has no login. Whoever can reach the
address it listens on can see the dashboard. Keep it on loopback or a private
network interface.

## Binary with systemd

Install the binary:

```sh
curl -LO https://github.com/lamplitkeep/sitrephq/releases/latest/download/sitrep-linux-x86_64
sudo install -m 755 sitrep-linux-x86_64 /usr/local/bin/sitrep
```

Create a user and a config directory:

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin sitrep
sudo mkdir -p /etc/sitrep
sudo cp config.yml /etc/sitrep/
sudo cp .env /etc/sitrep/.env
sudo chown -R root:sitrep /etc/sitrep
sudo chmod 750 /etc/sitrep
sudo chmod 640 /etc/sitrep/config.yml /etc/sitrep/.env
```

The config files are owned by root and readable by the `sitrep` group, so the
service can read them but can't change them. That matters for `config.yml` as
much as `.env`, since command sources run whatever the config says.

`/etc/systemd/system/sitrep.service`:

```ini
[Unit]
Description=SitRepHQ dashboard
After=network-online.target
Wants=network-online.target

[Service]
User=sitrep
Group=sitrep
WorkingDirectory=/etc/sitrep
EnvironmentFile=/etc/sitrep/.env
ExecStart=/usr/local/bin/sitrep /etc/sitrep/config.yml
Restart=on-failure
RestartSec=5

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now sitrep
journalctl -u sitrep -f
```

A few notes on the unit:

- `EnvironmentFile=` loads the secrets. SitRepHQ would also find `.env` in its
  working directory, but naming the file makes it explicit.
- If `bind` uses a VPN or tailnet address, that interface has to exist before
  SitRepHQ starts. `network-online.target` covers most setups. For Tailscale,
  add `After=tailscaled.service` as well.
- `ProtectHome=yes` hides home directories from the service. If you use
  command sources that need tools or credentials under a home directory (the
  aws CLI's `~/.aws`, for example), remove it or pass credentials through
  `.env` instead.
- To listen on a port below 1024, such as 443, add
  `AmbientCapabilities=CAP_NET_BIND_SERVICE` rather than running as root.

### Updating

```sh
curl -LO https://github.com/lamplitkeep/sitrephq/releases/latest/download/sitrep-linux-x86_64
sudo install -m 755 sitrep-linux-x86_64 /usr/local/bin/sitrep
sudo systemctl restart sitrep
```

`install` writes the new file and moves it into place, so the running process
is never left with a half-written binary.

## HTTPS

Plain HTTP on a private network is a reasonable starting point, but browsers
restrict HTTPS pages from framing HTTP content, so once any of your panes use
HTTPS the dashboard should too. There are three ways to get there.

### tailscale serve

If you use Tailscale, this is the least work. Leave `bind` on loopback and let
Tailscale provide the certificate and the tailnet-only listener:

```sh
sudo tailscale serve --bg 1986
```

The dashboard is then at `https://<machine>.<tailnet>.ts.net`, reachable only
from your tailnet.

### Reverse proxy

Leave out the `tls` block, keep `bind` on `127.0.0.1:1986`, and proxy to it.
Caddy:

```
sitrep.example.com {
    reverse_proxy 127.0.0.1:1986
}
```

Make sure the proxy itself only listens on your private interface or restricts
access by source address. Putting SitRepHQ behind a public reverse proxy without
authentication in front of it publishes the dashboard to the internet.

Log panes use server-sent events. Caddy handles them without configuration. With
nginx, turn off buffering for that path:

```nginx
location /api/queries/stream {
    proxy_pass http://127.0.0.1:1986;
    proxy_buffering off;
    proxy_http_version 1.1;
    proxy_set_header Connection "";
}
```

### Built-in TLS

SitRepHQ can serve HTTPS itself:

```yaml
bind: 100.64.0.5:443
tls:
  cert: /etc/sitrep/tls/fullchain.pem
  key: /etc/sitrep/tls/privkey.pem
```

A dashboard on a private address can't pass Let's Encrypt's HTTP challenge,
because Let's Encrypt can't reach it. Use the DNS challenge instead, with any
certbot or lego DNS plugin for your provider. Point a public DNS record at the
private address; it resolves for everyone but only connects from inside your
network.

Certificates are read at startup, so restart SitRepHQ after each renewal. With
certbot, a deploy hook can copy the new files somewhere the `sitrep` user can
read and restart the service:

```sh
#!/bin/sh
# /etc/letsencrypt/renewal-hooks/deploy/sitrep.sh
install -m 640 -g sitrep "$RENEWED_LINEAGE/fullchain.pem" /etc/sitrep/tls/fullchain.pem
install -m 640 -g sitrep "$RENEWED_LINEAGE/privkey.pem" /etc/sitrep/tls/privkey.pem
systemctl restart sitrep
```

## Docker

Two images are published to `ghcr.io/lamplitkeep/sitrephq`:

- `:latest` is distroless. It has no shell, so `command` sources can't run.
- `:full` is Debian slim with a shell, curl, and jq. Command sources work, but
  any other tool they call has to be added in your own image:

```dockerfile
FROM ghcr.io/lamplitkeep/sitrephq:full
RUN apt-get update && apt-get install -y --no-install-recommends awscli \
    && rm -rf /var/lib/apt/lists/*
```

`docker-compose.yml`:

```yaml
services:
  sitrep:
    image: ghcr.io/lamplitkeep/sitrephq:latest
    network_mode: host
    volumes:
      - ./config:/config:ro
    env_file: ./config/.env
    restart: unless-stopped
```

The image runs `/sitrep /config/config.yml`, so the config file must be named
`config.yml` at the top of the mounted directory. Mount the whole config
directory, not just `config.yml`. `theme.yml` and any
included source files are read from the same directory.

**Use host networking.** Inside a container, `127.0.0.1` is the container
itself, so a source like `base: http://127.0.0.1:2375` would point at nothing.
Host networking also lets `bind` use the host's VPN or tailnet address directly.
The catch is that only one container can listen on a given port, so two
SitRepHQ containers on one host need different `bind` ports.

If you publish ports instead of using host networking, publish only to a private
address (`-p 100.64.0.5:1986:1986`, not `-p 1986:1986`) and set `bind:
0.0.0.0:1986` inside the container. Be aware that ports published on a Tailscale
address break when Tailscale re-authenticates. `docker restart` doesn't fix it;
recreate the container with `docker compose up -d --force-recreate`.

## sitrep-agent

`systemd-agent` sources need `sitrep-agent` on each host you want units from.
It runs `systemctl show` for a list of units you give it and serves the results
as JSON at `/units`.

```sh
curl -LO https://github.com/lamplitkeep/sitrephq/releases/latest/download/sitrep-agent-linux-x86_64
sudo install -m 755 sitrep-agent-linux-x86_64 /usr/local/bin/sitrep-agent
```

It's configured with two environment variables:

| Variable | Default | Notes |
| --- | --- | --- |
| `AGENT_BIND` | `127.0.0.1:9100` | Address and port to listen on. Set it to the host's VPN or tailnet address so the dashboard can reach it. |
| `AGENT_UNITS` | `/etc/sitrep-agent/units` | The file listing which units to report |

### The units file

One unit per line. Blank lines and lines starting with `#` are ignored.

```
# /etc/sitrep-agent/units
nginx.service
postgresql.service
backup.timer
backup.service

# a target reports itself plus every .service it pulls in
ingest.target
```

Use full unit names, including the suffix. A `.target` line reports the target
and every `.service` unit in its dependency tree, so a group of related jobs can
be tracked by adding them to one target instead of listing each one here. New
services added to the target show up without editing this file.

The file is read on every request, so edits take effect on the dashboard's next
poll without restarting the agent. A unit that doesn't exist on the host shows as
ghost.

### systemd unit

`/etc/systemd/system/sitrep-agent.service`:

```ini
[Unit]
Description=SitRepHQ agent
After=network-online.target
Wants=network-online.target

[Service]
DynamicUser=yes
Environment=AGENT_BIND=100.64.0.20:9100
ExecStart=/usr/local/bin/sitrep-agent
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now sitrep-agent
curl -s http://100.64.0.20:9100/units
```

`DynamicUser=yes` runs it as a throwaway unprivileged user. Reading unit state
doesn't need root, and the agent can't start, stop, or change anything. The units
file only needs to be world-readable, which it is by default.

The agent has no authentication. Bind it to a private address, and firewall port
9100 so only the dashboard host can reach it. If you use Tailscale, add
`After=tailscaled.service` so the address exists before the agent binds it.

## Docker socket proxy

The `docker` source never talks to the Docker socket directly. Run a read-only
proxy on each Docker host:

```yaml
services:
  dockerproxy:
    image: tecnativa/docker-socket-proxy
    environment:
      CONTAINERS: 1
      POST: 0
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock:ro
    ports:
      - "100.64.0.10:2375:2375"
    restart: unless-stopped
```

`CONTAINERS: 1` allows listing containers and nothing else that matters. Publish
the port on a private address only. An exposed Docker API with write access is
root on that host.