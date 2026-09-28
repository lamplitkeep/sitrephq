# Configuration

SitRepHQ is configured with up to three files in one directory:

| File | Required | Holds |
| --- | --- | --- |
| `config.yml` | yes | Server settings, sources, and layout |
| `.env` | no | Secrets referenced by name from `config.yml` |
| `theme.yml` | no | Color, font, and shape overrides |

`sitrep` reads `./config.yml` by default. Pass a path to use another file:

```sh
sitrep /etc/sitrep/config.yml
```

`theme.yml` is always read from the same directory as the config file. `.env` is
read from the working directory the process was started in, which is usually the
same place. If you run under systemd, set `WorkingDirectory=` or use
`EnvironmentFile=` instead (see [deployment.md](deployment.md)).

The annotated example at `sitrep/examples/config.example.yml` covers every option
in comments. This page explains how the pieces fit together.

## Secrets

Config files never hold secret values. Any field ending in `_env` names an
environment variable, and SitRepHQ reads the value at startup:

```yaml
auth:
  bearer_env: DO_TOKEN
```

```sh
# .env
DO_TOKEN=dop_v1_...
```

Variables already present in the environment take priority over `.env`, so a
systemd `EnvironmentFile=` or a container's `--env-file` always wins. If a
variable a source needs is missing, startup stops with an error naming it. That
is deliberate: a dashboard that starts with a silently broken source is worse
than one that refuses to start.

Keep `.env` out of version control and readable only by the user running
SitRepHQ.

## Server

```yaml
bind: 127.0.0.1:1986
```

`bind` is the address and port the dashboard listens on. SitRepHQ has no login,
so this is the only thing standing between the dashboard and whoever can reach
it. Bind to loopback, or to a VPN or tailnet address when you want to view it
from other devices. Do not bind a public address.

```yaml
tls:
  cert: /etc/sitrep/fullchain.pem
  key: /etc/sitrep/privkey.pem
```

`tls` is optional. With it, SitRepHQ serves HTTPS itself. Without it, it serves
plain HTTP, which is what you want behind a reverse proxy. Certificates are read
once at startup, so restart after renewal.

## Sources

A source is something SitRepHQ polls. Every source has:

- `type`: one of the six types below
- `name`: unique across the file. It is the zone label in the status strip and
  the name tabs use to refer to it.
- `interval`: how often to poll, as a duration such as `30s`, `5m`, `6h`, or `1d`

Each source runs on its own schedule and its latest result is cached, so a slow
or broken source never delays the others. The page refreshes from that cache
every 30 seconds, which means intervals shorter than `30s` won't show up any
faster.

When a fetch fails, the zone is marked down, the error is shown in its tooltip,
and it is pinned to the attention bar until the next successful fetch.

Sources appear in the strip in the order they are listed.

### docker

```yaml
- type: docker
  name: docker-vps1
  base: http://203.0.113.10:2375
  interval: 30s
```

Lists every container on the host, running or not, one pill per container.
`base` should be a read-only Docker socket proxy such as
[tecnativa/docker-socket-proxy](https://github.com/Tecnativa/docker-socket-proxy)
with `CONTAINERS=1`. Never expose the Docker socket itself.

| Container | Pill |
| --- | --- |
| running, healthy or no health check | ok |
| running, health check starting | run |
| restarting or created | run |
| running, unhealthy | bad |
| exited | bad |
| anything else | ghost |

A container without a health check shows ok as long as it is running, which only
tells you the process is alive. See [healthchecks.md](healthchecks.md) for making
those pills mean something.

### systemd-agent

```yaml
- type: systemd-agent
  name: systemd-worker1
  url: http://198.51.100.20:9100/units
  interval: 30s
  label_rules:
    strip_prefix: ingest-
    strip_suffix: -pull
    aliases:
      nightly-backup: backup
```

Reads units from `sitrep-agent` running on another host.

| Unit | Pill |
| --- | --- |
| active | ok |
| activating or deactivating | run |
| inactive, last run succeeded | idle |
| target | idle |
| failed, or last run did not succeed | bad |
| not found or unknown | ghost |

The idle state is what makes timer-driven jobs readable: a oneshot service that
ran and exited cleanly is idle, not down. A failed unit stays bad until it runs
again successfully or you clear it with `systemctl reset-failed`.

`label_rules` shortens unit names for display. The prefix and suffix are removed
first, then `aliases` maps the result to a new label. The full unit name stays in
the pill's tooltip so you can still search for it. Label rules only apply to
`systemd-agent` sources.

### pihole

```yaml
- type: pihole
  name: dns
  base: http://192.168.1.53:8080
  password_env: PIHOLE_PASSWORD
  interval: 30s
```

Pi-hole v6 only. `base` is the address of the web interface without `/admin`.
Use an app password (Settings, Web interface / API) rather than your login
password.

Shows the percentage of queries blocked and the number of active clients. The
first `pihole` source in the file also feeds log panes (see
[panes.md](panes.md#log-panes)).

Pi-hole limits concurrent API sessions. SitRepHQ holds two, one for stats and one
for the query stream, and logs both out on shutdown. If Pi-hole starts rejecting
logins with 401 after many restarts, old sessions are still holding seats. They
expire on their own after 30 minutes, or you can raise
`webserver.api.max_sessions` in `pihole.toml`.

### unifi

```yaml
- type: unifi
  name: net
  base: https://192.168.1.1
  key_env: UNIFI_KEY
  gateway: Gateway
  insecure: true
  interval: 30s
```

Reads the UniFi Network integration API. The API key has to be created inside
the Network application itself, under Settings, Control Plane, Integrations. A key
from Site Manager or the console settings will not work.

| Field | Notes |
| --- | --- |
| `base` | The console's address |
| `site` | Optional. The first site is used when omitted. |
| `gateway` | Optional. The device name of your gateway, exactly as it appears in UniFi. Enables the WAN chart. |
| `insecure` | Accept self-signed certificates. UniFi consoles ship with one. |

Each device is a pill: ok when online, ghost when a firmware update is waiting,
bad when offline. A client count follows.

With `gateway` set, the zone also draws a 24 hour WAN throughput chart. The API
has no history, so SitRepHQ builds its own in memory from 5 minute averages. The
chart is empty after every restart and takes a day to fill.

### http-json

```yaml
- type: http-json
  name: forgejo
  url: https://git.example.com/api/v1/repos/owner/project
  interval: 15m
  auth:
    header: { name: Authorization, value: "token {}", value_env: FORGEJO_TOKEN }
  pills:
    - { path: open_pr_counter, label: "PRs {}", state: idle }
```

Fetches a URL and parses the response as JSON. Without `pills`, it shows a single
pill when the request succeeds.

`auth` takes one of three forms. Each block below is an alternative, not one
config:

```yaml
auth:
  bearer_env: TOKEN                  # Authorization: Bearer <secret>

auth:
  header: { name: X-Api-Key, value_env: TOKEN }
                                     # X-Api-Key: <secret>

auth:
  header: { name: Authorization, value: "token {}", value_env: TOKEN }
                                     # Authorization: token <secret>

auth:
  basic: { user: admin, password_env: PASSWORD }
```

The `value` template covers APIs that want a scheme other than Bearer. `{}` is
replaced by the secret. Without `value`, the header is the bare secret.

`insecure: true` accepts self-signed certificates.

`expect: ok` turns the source into a reachability check. Any successful response
counts and the body is ignored, which is useful for pages that don't return JSON.

### command

```yaml
- type: command
  name: disk-root
  run: >
    printf '{"used": %s}' $(df -B1 --output=used / | tail -1)
  interval: 5m
  pills:
    - { path: used, label: "root {}", format: bytes, state: idle }
```

Runs `run` with `sh -c` and reads standard output. Pipes, `$(...)`, and
environment variables all work. The command inherits SitRepHQ's environment,
so secrets from `.env` reach it without appearing in the config.

| Field | Default | Notes |
| --- | --- | --- |
| `timeout` | `30s` | The command is killed if it runs longer |
| `parse` | `json` | `json`, `number`, or `raw` (plain text) |

A non-zero exit or a timeout marks the source down.

Because this file can run arbitrary commands, anyone who can edit it can run code
as the SitRepHQ user. Give it the same permissions as `.env`.

The `:latest` Docker image has no shell, so command sources fail there. Use the
`:full` image or the binary.

## Pills

`http-json` and `command` sources can turn their response into any number of
pills:

```yaml
pills:
  - path: status.database
    label: "db {}"
    map:
      healthy: ok
      degraded: bad
  - path: disk.used
    label: "disk {}"
    format: bytes
    state: idle
```

| Field | Notes |
| --- | --- |
| `path` | Where the value is in the JSON. Dots separate keys. Numbers index arrays and negative numbers count from the end. |
| `label` | Pill text. `{}` is replaced by the value. |
| `format` | `bytes` (1024-based, so 8.4e12 shows as 7.6 TB), `percent`, or `number` (thousands separators) |
| `state` | A fixed state for the pill: `ok`, `idle`, `run`, `ghost`, or `bad` |
| `map` | Picks the state from the value. Values not in the map show as ghost. |

Use `state` for numbers you only want to read, like a bill or a bucket size.
`idle` is the usual choice because it doesn't draw the eye. Use `map` when the
value itself says whether things are fine.

If the path no longer exists in the response, the pill shows `?` as ghost. APIs
change, and a pill that goes quietly wrong is worse than one that says it can't
find its data.

<!-- TODO: confirm path syntax and the default state when neither state nor map is set -->

## Includes

Any source can live in its own file:

```yaml
sources:
  - !include sources/docker-vps1.yml
  - !include sources/dns.yml
  - type: docker
    name: docker-local
    base: http://127.0.0.1:2375
    interval: 30s
```

Each included file holds exactly one source as a plain mapping, with no leading
dash:

```yaml
# sources/dns.yml
type: pihole
name: dns
base: http://192.168.1.53:8080
password_env: PIHOLE_PASSWORD
interval: 30s
```

Include paths are relative to the config file. SitRepHQ refuses absolute paths,
paths that resolve outside the config directory, and files that don't end in
`.yml` or `.yaml`.

One source per file is intentional. It lets you copy a single preset, keep your
edits to it in one place, and compare it against upstream later. See
[presets.md](presets.md).

## Layout

The layout decides what the page shows beyond the status strip. There are two
shapes.

A single view shows every source in the strip and one set of panes:

```yaml
layout:
  panes:
    - { title: vps1, url: "https://vps1.example.com:7681", kind: term }
    - { title: blog, url: "https://blog.example.com" }
```

A tabbed layout turns each tab into its own view:

```yaml
layout:
  tabs:
    - name: OVERVIEW
      panes:
        - { title: blog, url: "https://blog.example.com" }
    - name: DNS
      sources: [dns, net]
      panes:
        - { title: queries, kind: log }
    - sources: [systemd-worker1]
      panes: []
```

| Field | Notes |
| --- | --- |
| `name` | Tab label. Unnamed tabs are labeled ALPHA, BRAVO, CHARLIE, and so on in order. |
| `sources` | The sources shown in this tab's strip. Leave it out to show all of them. A source can appear in more than one tab. |
| `panes` | The panes for this tab |

Tabs appear down the right edge. Keys 1 through 9 switch between them. A layout
with a single tab, or with flat `panes`, shows no tab rail at all.

The attention bar under the masthead always covers every source, whatever tab is
open. Each item in it is tagged with the tab that holds it, and clicking the tag
switches to that tab. You can split a large setup into focused tabs without
losing the one glance that tells you something broke.

Only the open tab's panes are loaded. Switching away closes them, including
terminal connections, and switching back loads them fresh.

If `tabs` has any entries, flat `panes` is ignored.

Pane kinds, filters, and troubleshooting are covered in [panes.md](panes.md).

## Themes

`theme.yml` overrides the dashboard's colors, fonts, and shapes. Every key is
optional:

```yaml
bg: "#14171c"
state-bad: "#e06c6c"
pill-radius: "999px"
```

`sitrep/examples/theme.example.yml` lists every key with a description, and three
complete palettes sit beside it. Quote color values, because an unquoted `#`
starts a YAML comment.

Unknown keys are ignored without warning, so check spelling if a change doesn't
appear. A `theme.yml` that isn't valid YAML stops startup.

The five state names are fixed. You can change what ok or bad look like, but not
add a sixth state.

## Checking a config

<!-- TODO: confirm sitrep check behavior and arguments -->

```sh
sitrep check
```

validates the config the same way startup does and reports on each pane's
target. Startup itself also validates, so a broken config fails immediately with
a message pointing at the problem rather than serving a half-working page.

Common errors:

| Message | Cause |
| --- | --- |
| `environment variable X not set` | A `_env` field names a variable that isn't in the environment or `.env` |
| `sources lists "x", but no source with that name is declared` | A tab refers to a source name that doesn't exist. The error lists the names that do. |
| `url must start with http:// or https://` | A pane URL uses another scheme |
| `include "...": resolves outside the config directory` | An include path climbs out of the config directory |
| `unknown variant` | A misspelled `type`, `kind`, `parse`, or `expect` value |