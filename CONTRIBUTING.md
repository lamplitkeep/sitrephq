# Contributing

Bug reports, fixes, and new presets are all welcome. For anything larger than a
fix, open an issue first so we can agree on the approach before you put time
into it.

## Layout

```
sitrephq/
├── sitrep/              the dashboard (crate: sitrephq, binary: sitrep)
│   ├── src/             server, config, sources
│   ├── static/          frontend: HTML, CSS, vanilla JS web components
│   ├── examples/        example config, env, and themes
│   └── presets/         ready-made source files
├── sitrep-agent/        the systemd agent
└── docs/
```

## Building and running

You need a recent stable Rust toolchain.

```sh
cargo build --workspace
```

To run the dashboard, point it at a config:

```sh
cp sitrep/examples/config.example.yml sitrep/config.yml
# edit it down to sources you can reach
cargo run -p sitrephq -- sitrep/config.yml
```

`sitrep/config.yml`, `.env`, and `theme.yml` are gitignored, so your local
setup stays out of commits.

In debug builds the frontend files are read from `sitrep/static/` on each
request, so a browser refresh picks up HTML, CSS, and JS changes without a
rebuild. Release builds embed them in the binary.

## Tests

```sh
cargo test --workspace
node --test sitrep/static/
```

The Rust tests cover config parsing, validation, and include confinement. The
Node tests cover the state classifiers, which decide what color every pill is.
If you change how a container or unit maps to a state, add a case for it.

Before opening a pull request:

```sh
cargo fmt --all
cargo clippy --workspace
```

## Ground rules

These keep the project what it is. A pull request that breaks one will be asked
to change.

- **Secrets never live in config.** New fields that need a credential take the
  name of an environment variable, ending in `_env`.
- **The five states are fixed.** ok, idle, run, ghost, bad. New sources map
  onto them; they don't add a sixth.
- **No proxying panes.** Panes are loaded by the viewer's browser. The server
  doesn't fetch, rewrite, or relay them.
- **No build step for the frontend.** Plain HTML, CSS, and JavaScript modules.
  No framework, no bundler, no npm dependencies at runtime.
- **Data from sources is untrusted.** Put it in the page with `textContent`,
  never `innerHTML`.
- **The user decides what they see.** Prefer a config option to a hardcoded
  choice about which fields to show or how to label them.

## New source types vs presets

Most integrations should be a preset, not new Rust code. If a service has a JSON
API or a CLI, an `http-json` or `command` source with the right pills covers it,
and anyone can maintain it without touching the codebase.

A new named source type is only worth it when a service needs something a
generic source can't do: a login handshake, merging several endpoints into one
view, or keeping state between polls. Open an issue to discuss it first.

## Adding a preset

See [docs/presets.md](docs/presets.md#writing-a-preset). In short: one source
per file, placeholders in angle brackets, a header comment with the service
version you tested against and the token scope it needs. Test it against a
real instance.

## Commits and sign-off

Keep each pull request to one change, and write commit messages that say why,
not just what.

Sign off your commits to certify you wrote the change or have the right to
submit it under the project's license, per the
[Developer Certificate of Origin](https://developercertificate.org):

```sh
git commit -s
```

This adds a `Signed-off-by:` line with your name and email. Pull requests
without it can't be merged.

## License

By contributing, you agree your contributions are licensed under the MIT
license, the same as the rest of the project.