# Security

## Reporting a vulnerability

Please don't open a public issue for a security problem.

Report it privately through GitHub: go to the repository's **Security** tab and
choose **Report a vulnerability**. Include what you found, how to reproduce it,
and the version or commit you tested.

You should get a response within a week. Once a fix is ready, it ships in a new
release and the advisory is published with credit to you, unless you'd rather
stay anonymous.

## Supported versions

Only the latest release gets security fixes.

## Security model

SitRepHQ is built to run on a private network, and its design assumes that.
Knowing what it does and doesn't protect against will help you decide whether
something is a vulnerability.

- **There is no login.** Anyone who can reach the listening address can see the
  dashboard. Access control is the network: bind to loopback or a VPN or
  tailnet address.
- **Secrets are never sent to the browser.** Config holds only the names of
  environment variables. `/api/config` serves layout, theme, and display
  settings, never source definitions or credentials.
- **Panes are not proxied.** The browser loads them directly. The server sends
  a Content-Security-Policy that limits frames to the configured pane origins,
  and `frame-ancestors 'none'` so the dashboard can't be framed.
- **Includes are confined.** `!include` paths can't be absolute or resolve
  outside the config directory.
- **Command sources run shell commands.** Whoever can edit the config can run
  code as the SitRepHQ user. The config file should be protected like the
  `.env` file.

## In scope

- Secrets or source definitions reaching the browser or any HTTP response
- Script injection through data returned by a source (container names, unit
  names, DNS query domains, API responses)
- Ways around the Content-Security-Policy or pane URL validation
- Includes escaping the config directory
- Anything that lets a network client change what the server does, polls, or
  runs

## Out of scope

- Access to a dashboard bound to a public address. That's a configuration
  choice the docs warn against.
- Command execution by someone who can already edit the config file
- Weaknesses in services SitRepHQ talks to (Pi-hole, UniFi, Docker proxies)
- Missing rate limiting or authentication, which are outside the design