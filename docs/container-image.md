# Container image

The repository ships a minimal container image for the REST API. The image runs
the same `stellar-contract-platform` binary as the release archives:

```text
stellar-contract-platform api serve --bind 0.0.0.0:8080
```

> **Publication status:** the release workflow is wired to build, smoke-test, and
> push the image to GHCR when a `v*.*.*` tag is pushed. Nothing in this
> repository has published an image yet; the first image appears only after such
> a tag runs successfully.

## Build

From the repository root:

```bash
docker build --tag stellar-contract-platform:local .
```

The build is multi-stage: `rust:1.88-bookworm` compiles the CLI with
`cargo build --release --locked`, and only the binary is copied into a
`debian:bookworm-slim` runtime. The runtime image runs as UID `10001` (user
`observatory`), listens on port `8080`, and has no build tools or source code.

## Run

```bash
docker run --rm --publish 8080:8080 stellar-contract-platform:local
curl --fail http://127.0.0.1:8080/api/v1/health
```

The health endpoint is public and answers `200` with `{"status":"ok"}`. A
`HEALTHCHECK` probes the same path every 30 seconds.

The committed releases use the tag as the image tag:

```bash
docker pull ghcr.io/tech-cred/stellar-contract-platform:v0.1.0
```

## Configuration

The binary reads flags, not an environment-driven config file. The image
defaults to `api serve --bind 0.0.0.0:8080`; override the command to change any
setting.

| Flag | Default | Purpose |
| ---- | ------- | ------- |
| `--bind <ADDR>` | `127.0.0.1:8080` (CLI); image `0.0.0.0:8080` | Listen address. |
| `--no-auth` | off | Disable authentication for local development only. Do not use in production. |
| `--key <ROLE:KEY>` | none | Register an API key. Repeatable. See secrets below. |
| `--rate-limit <N>` | config file, else `120` | Requests per window. |
| `--window-secs <N>` | `60` | Rate-limit window length. |
| `--max-connections <N>` | `64` | Concurrent in-flight connections. |
| `--read-timeout-secs <N>`, `--write-timeout-secs <N>` | `10` | Socket timeouts. |
| `--max-request-line-bytes`, `--max-header-line-bytes`, `--max-headers`, `--max-body-bytes` | `8 KiB`, `8 KiB`, `100`, `8 MiB` | HTTP bounds. |

A project configuration file (`observatory.yml`, `.yaml`, `.toml`, or `.json`)
is discovered in the working directory, which is `/app` in the image. Mount one
read-only to set authentication, rate limits, route limits, and CORS:

```bash
docker run --rm --publish 8080:8080 \
  --volume "$PWD/observatory.yml:/app/observatory.yml:ro" \
  stellar-contract-platform:local
```

See [configuration.md](configuration.md) for the schema.

Authentication is required by default (`api.require_auth: true`). The health,
readiness, version, metrics, and error-catalog endpoints stay public, but every
analysis route needs an API key. Either mount a configuration that disables
authentication for local use, or provide keys at run time as described below.

### Environment variables

`RUST_LOG` controls logging (`ENV RUST_LOG=info` in the image) and is the only
environment variable the container translates automatically. Other settings are
flags; an orchestrator that wants environment-driven configuration should map
its variables to these flags explicitly.

### Secrets

Nothing is baked into the image and no build argument carries a secret. API keys
must be supplied at run time. Because `--key` is a flag rather than an
environment variable, run the binary through a shell to expand a secret from the
environment:

```bash
docker run --rm --publish 8080:8080 \
  --env OBSERVATORY_DEVELOPER_KEY \
  --entrypoint /bin/sh \
  stellar-contract-platform:local \
  -c 'exec stellar-contract-platform api serve --bind 0.0.0.0:8080 \
        --key "developer:$OBSERVATORY_DEVELOPER_KEY"'
```

Source `OBSERVATORY_DEVELOPER_KEY` from your secret store (Kubernetes Secret,
Docker secret, CI secret) rather than a committed `.env` file. Passing a secret
as a command-line argument is visible to processes inside the container; prefer
secret files mounted by the orchestrator when that matters. An empty or missing
value makes `--key` fail, which is safer than starting without authentication
when `require_auth` is enabled.

## Publishing

`.github/workflows/release.yml` has an `image` job that runs on tag pushes:

1. checks out the repository with the same SHA-pinned checkout action as the
   other release jobs;
2. logs in to `ghcr.io` with `GITHUB_TOKEN` (`packages: write` for the job);
3. builds the image and tags it with the release tag;
4. starts the container and waits up to 30 seconds for
   `/api/v1/health` to return `200`;
5. pushes the image only after the smoke test passes.

The workflow uses the Docker CLI directly, so there are no third-party action
pins beyond the checkout step. The image is not signed or attested yet; the
release provenance attestation covers the archive assets, not the container.
