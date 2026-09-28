# Health checks

A pill is only as honest as the information behind it. This page covers how to
make Docker containers and systemd units report states you can trust.

## Docker

Without a health check, Docker only knows whether a container's main process is
running. A web server that's up but returning errors on every request, or a
database that started but won't accept connections, still shows ok.

A health check is a command Docker runs inside the container on a schedule. Its
exit code decides whether the container is healthy, and SitRepHQ reads that
result:

| Docker reports | Pill |
| --- | --- |
| running, `healthy` | ok |
| running, no health check | ok |
| running, `starting` | run |
| running, `unhealthy` | bad |
| restarting or created | run |
| exited | bad |

Some Docker versions leave the health field out when there's no check and
others report `none`. SitRepHQ treats both as no health check, never as
unhealthy.

Exited containers are always bad, including ones you stopped on purpose. If you
keep stopped containers around, remove them or move them to a host the
dashboard doesn't watch.

### Adding a health check

In a compose file:

```yaml
services:
  app:
    image: example/app
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://127.0.0.1:8080/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 20s
```

`start_period` gives the service time to start before failures count. During
it the pill shows run.

### Rules that save time

**Use `127.0.0.1`, not `localhost`.** Inside many images `localhost` resolves
to the IPv6 address `::1` first. If the service only listens on IPv4, the check
fails even though the service is fine.

**Use the container's own port, not the published one.** The check runs inside
the container. If compose maps `8081:80`, the check talks to port 80. Checking
8081 fails forever, and the container shows bad even though nothing is wrong
with it.

**Use a tool the image actually has.** Many slim and distroless images don't
include curl or wget. Check with `docker exec <container> which wget`. When
neither is available, the application often ships its own health command, and
that is the better choice anyway.

**Check that it works before trusting it.** Run the command yourself:

```sh
docker exec <container> wget -q --spider http://127.0.0.1:8080/health; echo $?
```

`0` is healthy. Then watch the container's health state settle:

```sh
docker inspect --format '{{.State.Health.Status}}' <container>
```

### Examples

PostgreSQL:

```yaml
healthcheck:
  test: ["CMD-SHELL", "pg_isready -U $${POSTGRES_USER} -d $${POSTGRES_DB}"]
  interval: 30s
  timeout: 5s
  retries: 3
```

The doubled `$$` stops compose from substituting the variable itself, so it's
read inside the container.

Redis:

```yaml
healthcheck:
  test: ["CMD", "redis-cli", "ping"]
  interval: 30s
  timeout: 5s
  retries: 3
```

Traefik needs the ping endpoint turned on in its own configuration and named in
the check:

```yaml
command:
  - --ping=true
healthcheck:
  test: ["CMD", "traefik", "healthcheck", "--ping"]
  interval: 30s
  timeout: 5s
  retries: 3
```

A generic web app with wget available:

```yaml
healthcheck:
  test: ["CMD", "wget", "-q", "--spider", "http://127.0.0.1:80/"]
  interval: 30s
  timeout: 5s
  retries: 3
```

Point web checks at a real health endpoint if the app has one. A check against
`/` passes as long as the server answers, even if the app behind it is broken.

## systemd

`systemd-agent` sources report unit state directly:

| Unit | Pill |
| --- | --- |
| active | ok |
| activating or deactivating | run |
| inactive after a successful run | idle |
| target | idle |
| failed, or the last run did not succeed | bad |
| not found or unknown | ghost |

### Timers and oneshot services

Scheduled jobs are usually a `.timer` that starts a `Type=oneshot` service. Most
of the time the service is inactive, which would look like down on most
dashboards. SitRepHQ shows it as idle when its last run succeeded, run while it
is running, and bad when the last run failed.

For that to work, the service's exit code has to mean something. A script that
exits 0 after an error hides the failure. Use `set -e` in shell scripts, or exit
non-zero explicitly when the job fails.

A failed unit stays bad until it succeeds on its next run. Once you've dealt
with the cause, clear it right away with:

```sh
sudo systemctl reset-failed <unit>
```

### Ghost units

A ghost pill for a systemd unit usually means the agent was asked about a unit
that doesn't exist on that host: it was renamed, removed, or never installed.
Either install it or stop tracking it.