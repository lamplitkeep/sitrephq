# Panes

Panes are the part of the page below the status strip: terminals, previews of
your own sites, and the Pi-hole query log.

```yaml
layout:
  panes:
    - { title: vps1, url: "https://vps1.example.com:7681", kind: term }
    - { title: blog, url: "https://blog.example.com" }
    - { title: queries, kind: log }
```

| Field | Notes |
| --- | --- |
| `title` | Shown in the pane's title bar |
| `url` | `http://` or `https://` only. Not used by log panes. |
| `kind` | `site` (default), `term`, or `log` |
| `filter` | Log panes only. See below. |

Every pane has a reload button in its title bar.

## How panes load

Site and terminal panes are iframes. **The browser viewing the dashboard loads
them, not the SitRepHQ server.** Nothing is proxied. That has two consequences:

1. A pane URL has to be reachable from every device you open the dashboard on.
   If your phone isn't on the VPN, its panes will be blank even though the
   server can reach the target fine.
2. The target decides whether it can be framed. Many sites and apps refuse by
   default, and SitRepHQ can't override that.

SitRepHQ sends a Content-Security-Policy listing the origins of your configured
panes as the only ones allowed in frames, plus `frame-ancestors 'none'` so the
dashboard itself can't be framed by another page. You never edit this policy; it
follows your config.

## Site panes

`kind: site` shows a live, scaled-down view of a page. The page is laid out at a
fixed width (1280px by default, set by `pane-render-width` in `theme.yml`) and
then scaled to fit the pane, so you see the whole page the way it looks on a
desktop rather than a cramped mobile layout.

Site panes are best for things you want to glance at: your own sites, a status
page, a Grafana panel. They are still live pages, and you can click into them.

## Terminal panes

`kind: term` shows a web terminal at full size, unscaled. SitRepHQ doesn't
include a terminal. Run one on the host you want to watch and point the pane at
it. [ttyd](https://github.com/tsl0922/ttyd) is the usual choice:

```sh
ttyd -p 7681 -i 127.0.0.1 btop
```

The command is whatever ttyd runs, so btop, htop, glances, `journalctl -f`, or a
tmux session all work the same way. ttyd is read-only unless started with
`-W`. Leave it read-only for a dashboard.

Bind ttyd to loopback and put it behind something that provides HTTPS on your
private network, such as `tailscale serve` or your reverse proxy. Terminals use
WebSockets, and some proxy setups that carry WebSockets over HTTP/2 cause the
connection to drop and reconnect in a loop. If a terminal pane flaps, that is
the first thing to check.

Terminal panes on tabs you aren't viewing are closed, not hidden, so background
terminals don't hold connections open. Switching back reconnects them.

## Log panes

`kind: log` shows a live stream of DNS queries from Pi-hole, newest at the top.
It needs a `pihole` source in the config and takes no `url`.

```yaml
panes:
  - { title: all queries, kind: log }
  - { title: iot vlan, kind: log, filter: { client: "192.168.30.*" } }
  - { title: blocked, kind: log, filter: { status: gravity } }
```

| Filter | Matches |
| --- | --- |
| `client` | Client IP. Exact, or a prefix ending in `*`. |
| `domain` | Queried domain, same rules. Only a trailing `*` works, so `ads.*` matches but `*.example.com` does not. |
| `status` | Pi-hole status such as `forwarded`, `cache`, `gravity`, or `denylist`. Exact, case does not matter. |

All filters on a pane must match for a row to show. A pane without a filter
shows every query and adds a dropdown of the clients it has seen, so you can
narrow it down by hand.

SitRepHQ polls Pi-hole once every two seconds no matter how many log panes are
open and applies each pane's filter on the server, so one pane per VLAN costs
nothing extra. Blocked queries are highlighted. Each pane keeps the most recent
200 rows.

Only the first `pihole` source in the config feeds log panes.

## When a pane is blank

If a pane hasn't loaded after 8 seconds, SitRepHQ covers it with a notice. The
cause is almost always one of the following, roughly in order of how often it
comes up.

**Try opening the URL in a new tab from the same device.**

- **It doesn't load there either.** The device can't reach the target. Check the
  VPN is connected on this device, that the hostname resolves, and that the
  service is running.
- **It loads, but shows a certificate warning.** Browsers won't show a page with
  an untrusted certificate inside a frame, and there's no prompt to click
  through. Use a trusted certificate on the target. Accepting the warning in a
  tab sometimes works for the rest of the session, but don't rely on it.
- **It loads fine in a tab.** The target is refusing to be framed, or the
  browser is blocking it. Keep going.

**Open the browser's developer console on the dashboard.**

- `Refused to display ... in a frame because it set 'X-Frame-Options' to 'deny'`
  (or `sameorigin`): the target sends `X-Frame-Options`. See below.
- `Refused to frame ... because an ancestor violates the following Content
  Security Policy directive: "frame-ancestors ..."`: the target sends its own
  CSP with `frame-ancestors`. See below.
- `Mixed Content`: the dashboard is served over HTTPS and the pane uses
  `http://`. Browsers block this outright. Serve the target over HTTPS too.
- `Refused to frame ... because it violates ... "frame-src ..."`: the pane's
  origin isn't in the dashboard's policy. This happens if the page inside the
  pane redirects to a different origin, such as a login page on another host.
  Point the pane at the final URL instead.

**A login page appears in the pane, or it loops back to one.** Session cookies
for apps behind a login (Authelia, Authentik, an app's own login) are often set
with `SameSite=Lax` or `Strict`, and browsers won't send them to a frame on a
different site. Hosting the dashboard and the app under the same parent domain
(`sitrep.example.com` and `grafana.example.com`) usually fixes this. Otherwise
exempt the route from your auth layer on the private network.

### Allowing the dashboard to frame your own services

The fix for both header problems is on the target: allow the dashboard's exact
origin, and nothing else.

```
Content-Security-Policy: frame-ancestors https://sitrep.example.com
```

and remove `X-Frame-Options`. Browsers that understand `frame-ancestors` ignore
`X-Frame-Options` when both are present, but removing it avoids surprises.

**Never use `frame-ancestors *`.** That lets any site on the internet frame the
service, which opens it up to clickjacking. Name the dashboard's origin, scheme
and port included.

Caddy:

```
grafana.example.com {
    header -X-Frame-Options
    header Content-Security-Policy "frame-ancestors https://sitrep.example.com"
    reverse_proxy 127.0.0.1:3000
}
```

nginx:

```nginx
location / {
    proxy_pass http://127.0.0.1:3000;
    proxy_hide_header X-Frame-Options;
    proxy_hide_header Content-Security-Policy;
    add_header Content-Security-Policy "frame-ancestors https://sitrep.example.com" always;
}
```

Hiding the upstream CSP header replaces the app's whole policy, not just
`frame-ancestors`. If the app ships a policy you want to keep, copy its other
directives into the `add_header` line.

Traefik, as a middleware applied to the target's router:

```yaml
http:
  middlewares:
    framable-by-sitrep:
      headers:
        frameDeny: false
        customFrameOptionsValue: ""
        contentSecurityPolicy: "frame-ancestors https://sitrep.example.com"
```

Some apps set these headers themselves rather than through a proxy (Grafana's
`allow_embedding`, for example). Check the app's own settings first.

Sites you don't control, like a bank or a search engine, will never load in a
pane. That's working as intended on their end.