# Presets

A preset is a ready-made source definition for a specific service. It's an
ordinary `http-json` or `command` source with the URL path, auth style, and
pills already worked out, so you only fill in your host and the name of your
token variable.

Presets are copied, not imported. You take the file, put it in your config
directory, and it's yours to edit. Nothing updates underneath you, and you can
change which fields show or what they're called without waiting on anyone.

## Using a preset

Presets live in `sitrep/presets/` in the repository, one source per file.

1. Copy the file into a `sources/` directory next to your `config.yml`:

   ```sh
   mkdir -p sources
   cp sitrep/presets/forgejo-repo.yml sources/
   ```

2. Edit the copy. Presets mark what you need to change with angle brackets:
   host, owner, repository, and so on. Rename the source if you'll have more
   than one of the same kind.

3. Include it from `config.yml`:

   ```yaml
   sources:
     - !include sources/forgejo-repo.yml
   ```

4. Add the token to `.env` under the variable name the preset uses, or change
   the `_env` field to a name you prefer.

Because each preset is one file and one source, you can take only the parts of
a service you care about. If upstream improves a preset later, `diff` your copy
against the new one and bring over what you want.

## Available presets

<!-- TODO: keep this table in sync with sitrep/presets/ -->

| File | Service | Shows |
| --- | --- | --- |
| `forgejo-version.yml` | Forgejo or Gitea | Server is up (version endpoint) |
| `forgejo-repo.yml` | Forgejo or Gitea | Open pull requests, open issues, and stars for one repository |

The example config also shows complete sources for DigitalOcean billing, an
Uptime Kuma status page, and an S3 bucket size through the aws CLI. Copy them
out as a starting point for your own.

### Forgejo and Gitea

Forgejo and Gitea use `token <secret>` in the Authorization header, not
`Bearer`:

```yaml
auth:
  header: { name: Authorization, value: "token {}", value_env: FORGEJO_TOKEN }
```

Things to know:

- A private repository returns **404, not 403**, when the token is missing or
  doesn't have `read:repository` scope. Forgejo hides the existence of private
  repositories from anyone who can't see them. If a repo preset shows the
  source as down with a 404 and the URL looks right, check the token's scope.
- The version endpoint is public. It answers even when your token is wrong, so
  it tells you the server is up and nothing about your credentials.
- The number of repositories an account has is only returned in a response
  header (`X-Total-Count`), not the body. Pills read the body, so that count
  isn't available.

## When a preset drifts

Presets are written against a specific version of each service's API. If a
service renames or moves a field, the affected pill shows `?` in the ghost
state instead of a wrong value, and it lands in the attention bar. Look at the
raw response with curl, update the `path` in your copy, and consider sending the
fix upstream.

## Writing a preset

A good preset:

- Is one source, as a bare mapping, with no leading dash.
- Uses placeholders in angle brackets for anything instance-specific, and an
  obvious `_env` variable name for the secret.
- Picks a few pills a person would actually want at a glance, not every field
  the API returns. The user can add more.
- Uses `state: idle` for values that are only informational and `map` for
  values that mean something is wrong.
- Starts with a short comment block: what the preset shows, the service version
  it was tested against, the token scope it needs, and any quirks like the ones
  above.

For example:

```yaml
# Forgejo / Gitea: one repository's open PRs, issues, and stars.
# Tested against Forgejo 15 and Gitea 1.22.
# Token scope: read:repository. A private repo returns 404 without it.
type: http-json
name: forgejo-<repo>
url: https://<host>/api/v1/repos/<owner>/<repo>
interval: 15m
auth:
  header: { name: Authorization, value: "token {}", value_env: FORGEJO_TOKEN }
pills:
  - { path: open_pr_counter, label: "PRs {}", state: idle }
  - { path: open_issues_count, label: "issues {}", state: idle }
  - { path: stars_count, label: "stars {}", state: idle }
```

Pull requests with new presets are welcome. Test against a real instance and
note the version in the header comment.