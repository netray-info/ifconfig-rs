# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.17.0] - 2026-03-14

### Added

#### API Endpoints
- `GET /asn/{number}` — ASN lookup by number, returning org name, ASN category, network role, and anycast flag.
- `GET /range?cidr=<prefix>` — Network classification for an arbitrary CIDR prefix.
- `POST /diff` — Side-by-side enrichment comparison for two IPs; body: `{"a":"<ip>","b":"<ip>"}`.
- `GET /host` — Dedicated reverse-DNS hostname endpoint (previously only available via `/all`).
- `GET /isp` — Dedicated ISP/ASN endpoint (previously only available via `/all`).

#### Query Parameters
- `?format=<json|yaml|toml|csv>` — Format alias equivalent to a path suffix; works on all endpoints.
- `?lang=<BCP-47>` — Locale-aware city and country names (e.g. `?lang=de`).

#### Network Classification
- `is_anycast: bool` — Anycast detection via ASN heuristics; present in `network` object and `/asn/{number}` response.
- `is_cins: bool` — CINS Army bad-actor IP list detection; new `cins_army_ips` data file config key.
- `iana_label: string|null` — IANA special-purpose registry label for the address (e.g. "Shared Address Space").

#### Data Enrichment Modules
- `src/backend/cins.rs` — CINS Army IP list loader and matcher.
- `src/backend/iana.rs` — IANA special-purpose registry table lookup.

#### Response Headers / `/headers` Endpoint
- `x_forwarded_for_chain` field added to `/headers` JSON response, exposing the parsed XFF hop list.

#### CLI
- `--check` flag — validates all configured data files and exits with code 0 (all ok) or 1 (any failure). Useful in deploy scripts and container startup checks.

#### Frontend
- Share button — uses `navigator.share` when available; falls back to clipboard copy of the `?ip=` URL.
- Collapsible raw JSON per info card — `{·}` toggle on Network, Location, and User Agent cards.
- `?ip=` LRU response cache — in-memory cache for repeated arbitrary IP lookups (default: 5 min TTL, 1024 entries, configurable via `[cache]` section).

#### Configuration
- `[cache]` section with `enabled`, `ttl_secs`, and `max_entries` keys.
- `cins_army_ips` data file config key.

#### Observability
- `data_file_age_seconds` Prometheus gauge emitted for each loaded enrichment source.
- Tracing instrumentation added to backend hot-paths.

### Changed

- CLI auto-detection extended to `python-httpx` and `python-requests` (in addition to `curl`, `wget`, `httpie`).
- `/meta` `data_sources` entries now include a per-source `_updated` ISO-8601 timestamp sourced from file mtime; `null` when not loaded or mtime unavailable.
- Migrated to `netray-common` telemetry and static handler modules.
- CI: pinned action SHAs, switched from `cargo-audit` to `cargo-deny`, fixed SBOM toolchain step.
- Dev tooling: added `rust-toolchain.toml`, `Dockerfile.dev`, and frontend ESLint config.

## [0.16.0] - 2026-03-14

Initial tracked release.
