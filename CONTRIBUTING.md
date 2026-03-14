# Contributing to ifconfig-rs

## Prerequisites

- Rust (stable, edition 2021)
- Node.js 22+ and npm
- Docker (for integration tests)

## Data Directory Setup

The server requires several data files in a `data/` directory. These are not checked into the repository.

```sh
mkdir -p data
```

**Recommended files** (GeoIP and UA data are optional — the service starts without them, but geolocation, ISP, and User-Agent fields return `null`. If a path is configured but the file is missing or corrupt, the service will refuse to start.):

| File | Source |
|------|--------|
| `data/GeoLite2-City.mmdb` | [MaxMind GeoLite2](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) (free account required) |
| `data/GeoLite2-ASN.mmdb` | Same as above |
| `data/regexes.yaml` | [ua-parser/uap-core](https://github.com/ua-parser/uap-core/blob/master/regexes.yaml) |

**Optional enrichment files** (server runs without these, features degrade gracefully):

| File | Source |
|------|--------|
| `data/tor_exit_nodes.txt` | [Tor Project bulk exit list](https://check.torproject.org/torbulkexitlist) |
| `data/cloud_provider_ranges.jsonl` | Cloud provider IP range data (AWS, GCP, Azure, etc.) |
| `data/vpn_ranges.txt` | VPN provider CIDR ranges |
| `data/datacenter_ranges.txt` | Datacenter CIDR ranges |
| `data/bot_ranges.jsonl` | Bot IP ranges (Googlebot, Bingbot, etc.) |
| `data/feodo_botnet_ips.txt` | [Feodo Tracker](https://feodotracker.abuse.ch/downloads/ipblocklist.txt) |
| `data/spamhaus_drop.txt` | [Spamhaus DROP](https://www.spamhaus.org/drop/drop.txt) |

## Building

```sh
# Build the frontend first (required — assets are embedded at compile time)
cd frontend && npm ci && npm run build && cd ..

# Build the Rust backend
cargo build
```

## Running Locally

```sh
cargo run -- ifconfig.dev.toml
# Server starts on http://127.0.0.1:8080
# Admin/metrics on http://127.0.0.1:9090 (if configured)
```

## Testing

```sh
# Unit tests only (fast, no network or external services)
cargo test --lib --no-fail-fast

# Unit + in-process integration tests
cargo test --no-fail-fast

# Docker-based integration tests
make integration

# Playwright E2E tests (requires a running server)
make acceptance

# All tests
make test

# Lint
cargo clippy

# Format check
cargo fmt -- --check
```

## Pull Request Workflow

1. Create a feature branch from `master`.
2. Make your changes in small, focused commits.
3. Run `cargo clippy` and `cargo test --no-fail-fast` before pushing.
4. Open a PR against `master`.
5. CI runs check, clippy, fmt, unit tests, integration tests, and Docker tests.

## Code Style

- Follow existing patterns in the codebase.
- Keep changes scoped — don't modify unrelated modules.
- Don't mix formatting-only changes with functional changes.
- See `CLAUDE.md` for detailed engineering principles.
