# Changelog

Notable changes to SitRepHQ are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/). Until 1.0, minor versions may
change the config format; those changes are called out under **Changed**.

## [Unreleased]

## [0.1.0] - 2026-10-07

First public release.

### Added

- Status strip with five states (ok, idle, run, ghost, bad) and per-zone counts
- Attention bar pinning failed, unknown, and unreachable items from every tab
- Sources: `docker` (through a socket proxy), `systemd-agent`, `pihole`
  (Pi-hole v6), `unifi` (UniFi Network integration API), `http-json`, and
  `command`
- Pills that pull values from JSON responses, with labels, formatting, fixed
  states, and value-to-state maps
- Label rules for shortening systemd unit names
- `!include` for splitting sources into one file each, confined to the config
  directory
- Panes: scaled site previews, web terminals, and a live Pi-hole query log with
  per-pane client, domain, and status filters
- Tabs, each with its own sources and panes, switched with keys 1 to 9
- 24 hour WAN throughput chart for UniFi gateways
- Theme overrides through `theme.yml`, with Binder, Ambient, and Situation
  palettes
- Content-Security-Policy generated from configured pane origins
- Optional built-in TLS
- `sitrep-agent` for reporting systemd units from remote hosts
- Forgejo and Gitea presets
- Graceful shutdown on SIGINT and SIGTERM, logging out of Pi-hole sessions
- `sitrep check` for validating a config and diagnosing panes
  - Release binaries for Linux x86_64 (`sitrep` and `sitrep-agent`), Docker
    images (`:latest` distroless and `:full`), and `cargo install sitrephq`

[Unreleased]: https://github.com/lamplitkeep/sitrephq/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lamplitkeep/sitrephq/releases/tag/v0.1.0