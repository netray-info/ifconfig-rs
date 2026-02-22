# CLAUDE.md — ifconfig-rs

## Rules

- Do NOT add a `Co-Authored-By` line for Claude in commit messages.
- Don't add heavy dependencies for minor convenience — check if existing deps already cover the need.
- Don't mix formatting-only changes with functional changes in the same commit.
- Don't modify unrelated modules "while you're in there" — keep changes scoped.
- Don't add speculative flags, config options, or abstractions without a current caller.
- Don't bypass failing checks (`--no-verify`, `#[allow(...)]`) without explaining why.
- Don't hide behavior changes inside refactor commits — separate them.
- Don't include PII, real email addresses, or real domains (other than example.com) in test data, docs, or commits.
- If uncertain about an implementation detail, leave a concrete `TODO("reason")` rather than a hidden guess.

## Engineering Principles

- **Performance**: Prioritize efficient algorithms and data structures. Avoid unnecessary allocations and copies.
- **Rust patterns**: Use idiomatic Rust constructs (enums, traits, iterators) for clarity and safety. Leverage type system to prevent invalid states.
- **KISS**: Simplest solution that works. Three similar lines beat a premature abstraction.
- **YAGNI**: Don't build for hypothetical future requirements — solve the current problem.
- **DRY + Rule of Three**: Tolerate duplication until the third occurrence, then extract.
- **SRP**: Each module/struct has one reason to change. Split when responsibilities diverge.
- **Fail Fast**: Validate at boundaries, return errors early, don't silently swallow failures.
- **Secure by Default**: Sanitize external input, no PII in logs, prefer safe APIs.
- **Reversibility**: Prefer changes that are easy to undo. Small commits over monolithic ones.

## Project Overview

**ifconfig-rs** is a "what's my IP" web service written in Rust, powering **ip.pdt.sh**. Returns IP address, hostname, geolocation, ISP, and user agent info as plain text, JSON, YAML, TOML, CSV, or via a SolidJS SPA depending on the client.

- **Author**: Lukas Pustina | **License**: MIT | **Edition**: 2021
- **Repository**: https://github.com/lukaspustina/ifconfig-rs

## Build & Test

```sh
cd frontend && npm ci && npm run build  # Build frontend (required before cargo build)
cargo build                  # Build (requires frontend/dist/)
cargo test --lib --no-fail-fast  # Unit tests (fast, no network)
cargo test                   # All tests including integration
cargo clippy                 # Lint
cargo fmt                    # Format
cargo run -- ifconfig.dev.toml  # Local dev server on :8080
make frontend-build          # Build frontend via make
make dev                     # Run dev server
make test                    # Unit + Docker integration + Playwright E2E
make integration             # Docker-based integration tests only
make acceptance              # Playwright E2E tests only
make bench                   # Run Criterion benchmarks
make docker-build            # Production Docker image
cargo bench                  # Run benchmarks directly (negotiation, asn_heuristic, serialization, cloud_lookup)
```

### Test Guidelines

- `cargo test --lib` is the fast reliable check — no network or external services needed (~168 unit tests).
- `cargo test` also runs integration tests in `tests/ok_handlers.rs` (~124 tests covering all endpoints, content types, `?ip=` lookups, `?fields=` filtering, batch, `/ipv6`, security headers, OpenAPI, and `/docs`), `tests/error_handler.rs`, `tests/rate_limit.rs` (5 tests covering rate limit headers, 429 behavior, and probe exemptions), and `tests/admin.rs` (admin port bearer auth). Total: ~300 tests.
- Integration tests spawn real TCP listeners with hyper_util for each test case.
- Docker integration tests (`make integration`) build and test inside a container via `tests/Dockerfile.tests`.
- Playwright E2E tests (`make acceptance`) use configurable `baseURL` (default `http://127.0.0.1:8000`, override via `BASE_URL` env var) across Chromium, Firefox, and WebKit.

## Architecture

```
Request → CompressionLayer → request_id → record_metrics → TraceLayer (request logging)
        → Middleware (requester info extraction, rate limiting, GeoIP date headers, CORS, security headers)
        → Router (content negotiation via negotiate()) → Handlers → Response (text/json/yaml/toml/csv/html)
```

**Backend**: Axum 0.8 with tower middleware, tokio runtime.
**Frontend**: SolidJS 1.9 SPA built with Vite 6, embedded via rust-embed for single-binary deployment.

Key modules:
- `src/lib.rs` — Module hub, `build_app()` returns `AppBundle` (main app + optional admin app), installs Prometheus metrics recorder, configures middleware stack (compression, request ID, metrics, tracing, CORS)
- `src/main.rs` — tokio entry point, config loading, `--print-config` flag, `IFCONFIG_LOG_FORMAT=json` support, optional admin port, SIGHUP reload, optional filesystem watcher (`watch_data_files`), graceful shutdown
- `src/config.rs` — `Config` struct (derives `Serialize` + `Deserialize`) loaded from config file + `IFCONFIG_` env vars via `config` crate
- `src/state.rs` — `AppState` wrapping Arc'd backends (GeoIP, UA parser, Tor nodes); `KeyedRateLimiter` uses `StateInformationMiddleware` for burst-capacity tracking; `trusted_proxies: Arc<Vec<IpNetwork>>` for CIDR-aware XFF parsing
- `src/backend/mod.rs` — Core logic: `get_ifconfig()` orchestrates GeoIP, reverse DNS, UA parsing, network classification
- `src/backend/user_agent.rs` — UA parsing wrapper around `uaparser`
- `src/backend/asn_heuristic.rs` — ASN name-based classification (hosting/VPN detection by ISP name). Covers ~33 named hosting providers + generic keyword patterns (including Google LLC, Hetzner, DigitalOcean, etc.) and ~12 VPN providers (including Mullvad via "31173 Services AB" alias). Matching is case-insensitive substring on ASN org name.
- `src/backend/cloud_provider.rs` — Cloud provider CIDR matching (AWS, GCP, Azure, Cloudflare, etc.)
- `src/backend/vpn.rs` — VPN range CIDR matching
- `src/backend/bot.rs` — Bot IP range matching (Googlebot, Bingbot, etc.)
- `src/backend/datacenter.rs` — Datacenter IP range matching
- `src/backend/feodo.rs` — Feodo C2 botnet IP matching
- `src/backend/spamhaus.rs` — Spamhaus DROP/EDROP threat list matching
- `src/enrichment.rs` — `EnrichmentContext` struct with `ArcSwap` hot-reload via SIGHUP; emits `enrichment_sources_loaded` gauges on load/reload; populates `missing_optional: Vec<&'static str>` with names of configured-but-failed optional sources and emits a single consolidated `WARN` summary line
- `src/routes.rs` — Axum router with explicit handler functions, `dispatch_standard()` compute-once dispatch, batch handler, OpenAPI spec via utoipa, Scalar API docs UI at `/docs`, static file serving via rust-embed
- `src/scalar_docs.html` — Lightweight HTML page for Scalar API reference UI (loaded via `include_str!()`; CDN dependency is client-side only)
- `src/handlers.rs` — Per-endpoint `to_json`/`to_plain` functions used as fn pointers by `dispatch_standard()`
- `src/negotiate.rs` — Content negotiation: format suffix → CLI detection → Accept header → HTML default
- `src/extractors.rs` — `RequesterInfo` extraction (IP from ConnectInfo/XFF with CIDR-aware trusted proxy matching, UA, URI)
- `src/middleware.rs` — Request ID generation (`X-Request-Id`), application metrics (`record_metrics`), security headers, cache control, rate limiting with `X-RateLimit-*` / `Retry-After` headers, `X-GeoIP-Database-Date` / `X-GeoIP-Database-Age-Days` headers
- `src/error.rs` — `AppError` enum with `IntoResponse` impl; `ErrorResponse` struct (JSON `{error, status}` body) and `error_response()` helper used by all error paths
- `src/format.rs` — `OutputFormat` enum with serialization to JSON/YAML/TOML/CSV/plain

**Content negotiation priority**: Format suffix (`/ip/json`) → CLI detection (curl/wget/httpie + Accept: */*) → Accept header → HTML (serve SPA).

**API endpoints**: `/`, `/ip`, `/ip/cidr`, `/tcp`, `/host`, `/location`, `/isp`, `/network`, `/user_agent`, `/all`, `/headers`, `/ipv4`, `/ipv6`, `/meta`, `/health`, `/ready` — all (except probes, `/ip/cidr`, and `/meta`) support format suffixes (`/json`, `/yaml`, `/toml`, `/csv`) and Accept header negotiation. `/ip/cidr` returns plain text only (`{ip}/32` or `{ip}/128`). `/meta` returns JSON site metadata (site name, version) used by the SPA. `/health` is a liveness probe; `/ready` is a readiness probe that checks GeoIP database availability and returns a `warnings` array for any configured-but-failed optional sources. `/docs` serves the Scalar API reference UI. `/api-docs/openapi.json` serves the OpenAPI spec.

**Batch endpoint**: `POST /batch` (and `/batch/{format}`) accepts a JSON array of IP addresses and returns enrichment results for each. Disabled by default (`batch.enabled = true` in config). N IPs consume N rate-limit tokens. Supports `?fields=` and `?dns=true`.

**Query parameters**: Most endpoints support `?ip=` (look up an arbitrary global IP instead of the caller's), `?fields=` (comma-separated top-level field names to include in response), and `?dns=true` (opt-in PTR lookup for `?ip=` queries; PTR is skipped by default for arbitrary IPs). For `?ip=` queries, `tcp` and `host` are omitted from the response (port is synthetic, PTR is slow and usually unwanted).

**OpenAPI**: Spec served at `GET /api-docs/openapi.json` via utoipa. All public endpoints are annotated with `#[utoipa::path]` with descriptions, `?ip=`/`?fields=`/`?dns=` params, and error responses (`body = ErrorResponse`). Response types derive `ToSchema` with `#[schema(example = ...)]` on all fields. The spec version is patched at runtime from `CARGO_PKG_VERSION` (no manual sync needed). Interactive docs via Scalar UI at `GET /docs`.

## Frontend

SolidJS 1.9 SPA in `frontend/`:
- `src/App.tsx` — Main component with data fetching; footer links to GitHub, `/docs` (API reference), and author
- `src/components/` — IpDisplay, InfoCards, ApiExplorer, RequestHeaders, Faq, ThemeToggle
- `src/lib/api.ts` — Fetches `/json` endpoint
- `src/lib/types.ts` — TypeScript interfaces matching Rust `Ifconfig` struct
- `src/styles/global.css` — Dark-mode-first design with CSS custom properties

Dev: `cd frontend && npm run dev` (proxies API calls to localhost:8080, including `/docs` and `/api-docs`).
Build: `cd frontend && npm run build` (outputs to `frontend/dist/`).

## Configuration

Config files: `ifconfig.dev.toml` (local dev), `ifconfig.example.toml` (all options documented).
Runtime config via a TOML file with `IFCONFIG_` env var overrides (`_` separates prefix from key, `__` separates nested sections):

```toml
base_url = "localhost"
project_name = "ifconfig-rs"
geoip_city_db = "data/GeoLite2-City.mmdb"
geoip_asn_db = "data/GeoLite2-ASN.mmdb"
user_agent_regexes = "data/regexes.yaml"
tor_exit_nodes = "data/tor_exit_nodes.txt"
cloud_provider_ranges = "data/cloud_provider_ranges.jsonl"
feodo_botnet_ips = "data/feodo_botnet_ips.txt"
vpn_ranges = "data/vpn_ranges.txt"
datacenter_ranges = "data/datacenter_ranges.txt"
bot_ranges = "data/bot_ranges.jsonl"
spamhaus_drop = "data/spamhaus_drop.txt"
# filtered_headers = ["^x-koyeb-", "^cf-"]
# watch_data_files = false

[server]
bind = "127.0.0.1:8080"
# admin_bind = "127.0.0.1:9090"  # Optional: Prometheus /metrics + /health
# admin_token = "change-me"      # Bearer token for admin port; unauthenticated if unset
# trusted_proxies = ["10.0.0.0/8"]
# cors_allowed_origins = ["*"]   # Default: allow all; handles OPTIONS preflight

[rate_limit]
per_ip_per_minute = 60
per_ip_burst = 10

[batch]
enabled = true     # disabled by default
max_size = 100     # max IPs per batch request
```

`watch_data_files = true` enables filesystem watcher for auto-reload of data files (alternative to SIGHUP).

Env var examples: `IFCONFIG_SERVER__BIND=0.0.0.0:8080`, `IFCONFIG_BASE_URL=ip.pdt.sh`, `IFCONFIG_SERVER__ADMIN_TOKEN=secret`.
Structured JSON logging: `IFCONFIG_LOG_FORMAT=json`.
Print effective config and exit: `--print-config` flag.

Config is validated at load time (`Config::validate()`) — zero rate-limit values are rejected with a descriptive error before the server starts.

GeoIP data in `data/`: `GeoLite2-City.mmdb`, `GeoLite2-ASN.mmdb`.

## CI/CD

GitHub Actions: check → clippy → fmt → build/test → Docker integration tests. All CI jobs (except fmt) build the frontend before cargo operations. Pushing to `prod` branch auto-builds and pushes Docker image to GHCR (`ghcr.io/lukaspustina/ifconfig-rs:latest`).

## Common Patterns

- Routes use explicit handler functions with `dispatch_standard()` for compute-once dispatch. Each handler module in `handlers.rs` exposes `to_json(&Ifconfig) -> Option<Value>` and `to_plain(&Ifconfig) -> String` fn pointers.
- `Ifconfig` struct in `backend/mod.rs` is the central data model — all endpoint responses derive from it. `tcp` is `Option<Tcp>` (null for `?ip=` queries where the port is synthetic). `Location` includes `region`, `region_code`, `postal_code`, `is_eu`, and `accuracy_radius_km` from GeoIP. `Network` struct holds IP classification (type, provider, flags).
- CLI client detection in `negotiate.rs` checks User-Agent patterns and `Accept: */*` header.
- Config values are loaded from a TOML file (`ifconfig.dev.toml` for local dev) via the `config` crate with env var overrides.
- `AppState` is shared via Axum's `State` extractor; all backends are `Arc`-wrapped.
- `build_app()` returns `AppBundle { app, admin_app }` — `admin_app` is `Some` only when `server.admin_bind` is configured and the metrics recorder installs successfully. In tests, multiple `build_app()` calls silently skip metrics (global recorder can only be set once).
- Rate limit middleware emits `X-RateLimit-Limit`, `X-RateLimit-Remaining` on all responses and `Retry-After` on 429s. `/health`, `/ready`, and `/batch` are exempt (batch has its own per-IP token consumption: N IPs = N tokens).
- Response compression via `CompressionLayer` (gzip) — outermost layer, respects `Accept-Encoding`.
- `X-Request-Id` header on every response — propagates client-sent IDs, otherwise generates 16-char hex IDs via atomic counter + random seed. Included in `TraceLayer` spans for log correlation.
- CORS via `tower_http::cors::CorsLayer` — configurable origins (default `["*"]`), handles OPTIONS preflight.
- Application-level Prometheus metrics: `http_requests_total{method,status}`, `http_request_duration_seconds{method}`, `enrichment_sources_loaded{source}`, `geoip_database_age_seconds`. `metrics` macros are no-op when no recorder is installed (safe in tests).
- Frontend assets are embedded at compile time via `rust-embed` — `cargo build` requires `frontend/dist/` to exist.
- All error responses are structured JSON via `error_response()` returning `ErrorResponse { error, status }`. The `ErrorResponse` struct derives `utoipa::ToSchema` and is referenced in OpenAPI error response annotations.
- Criterion benchmarks in `benches/` cover negotiation, ASN classification, serialization (all 4 formats), and cloud CIDR lookup. Run with `cargo bench` or `make bench`.
