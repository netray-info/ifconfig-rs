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
make frontend                # Build frontend via make
make dev                     # Run dev server
make tests                   # Unit + Docker integration + Playwright E2E
make integration             # Docker-based integration tests only
make acceptance              # Playwright E2E tests only
make docker-build            # Production Docker image
```

### Test Guidelines

- `cargo test --lib` is the fast reliable check — no network or external services needed.
- `cargo test` also runs integration tests in `tests/ok_handlers.rs` (74 tests covering all endpoints and content types), `tests/error_handler.rs`, and `tests/rate_limit.rs` (5 tests covering rate limit headers, 429 behavior, and probe exemptions).
- Integration tests spawn real TCP listeners with hyper_util for each test case.
- Docker integration tests (`make integration`) build and test inside a container via `tests/Dockerfile.tests`.
- Playwright E2E tests (`make acceptance`) run against production at `https://ip.pdt.sh` across Chromium, Firefox, and WebKit.

## Architecture

```
Request → Middleware (security headers, requester info extraction)
        → Router (content negotiation via negotiate()) → Handlers → Response (text/json/yaml/toml/csv/html)
```

**Backend**: Axum 0.8 with tower middleware, tokio runtime.
**Frontend**: SolidJS 1.9 SPA built with Vite 6, embedded via rust-embed for single-binary deployment.

Key modules:
- `src/lib.rs` — Module hub, `build_app()` returns `AppBundle` (main app + optional admin app), installs Prometheus metrics recorder
- `src/main.rs` — tokio entry point, config loading, `--print-config` flag, `IFCONFIG_LOG_FORMAT=json` support, optional admin port, graceful shutdown
- `src/config.rs` — `Config` struct (derives `Serialize` + `Deserialize`) loaded from config file + `IFCONFIG_` env vars via `config` crate
- `src/state.rs` — `AppState` wrapping Arc'd backends (GeoIP, UA parser, Tor nodes); `KeyedRateLimiter` uses `StateInformationMiddleware` for burst-capacity tracking
- `src/backend/mod.rs` — Core logic: `get_ifconfig()` orchestrates GeoIP, reverse DNS, UA parsing
- `src/backend/user_agent.rs` — UA parsing wrapper around `uaparser`
- `src/routes.rs` — Axum router with endpoint macros, static file serving via rust-embed
- `src/handlers.rs` — Macro-generated response formatters for each content type
- `src/negotiate.rs` — Content negotiation: format suffix → CLI detection → Accept header → HTML default
- `src/extractors.rs` — `RequesterInfo` extraction (IP from ConnectInfo/XFF, UA, URI)
- `src/middleware.rs` — Security headers, cache control, rate limiting with `X-RateLimit-*` / `Retry-After` headers
- `src/error.rs` — `AppError` enum with `IntoResponse` impl
- `src/format.rs` — `OutputFormat` enum with serialization to JSON/YAML/TOML/CSV/plain

**Content negotiation priority**: Format suffix (`/ip/json`) → CLI detection (curl/wget/httpie + Accept: */*) → Accept header → HTML (serve SPA).

**API endpoints**: `/`, `/ip`, `/tcp`, `/host`, `/location`, `/isp`, `/network`, `/user_agent`, `/all`, `/headers`, `/ipv4`, `/ipv6`, `/health`, `/ready` — all (except probes) support format suffixes (`/json`, `/yaml`, `/toml`, `/csv`) and Accept header negotiation. `/health` is a liveness probe; `/ready` is a readiness probe that checks GeoIP database availability.

## Frontend

SolidJS 1.9 SPA in `frontend/`:
- `src/App.tsx` — Main component with data fetching
- `src/components/` — IpDisplay, InfoCards, ApiDocs, ThemeToggle
- `src/lib/api.ts` — Fetches `/json` endpoint
- `src/lib/types.ts` — TypeScript interfaces matching Rust `Ifconfig` struct
- `src/styles/global.css` — Dark-mode-first design with CSS custom properties

Dev: `cd frontend && npm run dev` (proxies API calls to localhost:8080).
Build: `cd frontend && npm run build` (outputs to `frontend/dist/`).

## Configuration

Config files: `ifconfig.dev.toml` (local dev), `ifconfig.example.toml` (all options documented).
Runtime config via a TOML file with `IFCONFIG_` env var overrides (separator: `__`):

```toml
base_url = "localhost"
project_name = "ifconfig-rs"
geoip_city_db = "data/GeoLite2-City.mmdb"
geoip_asn_db = "data/GeoLite2-ASN.mmdb"
user_agent_regexes = "data/regexes.yaml"
tor_exit_nodes = "data/tor_exit_nodes.txt"

[server]
bind = "127.0.0.1:8080"
# admin_bind = "127.0.0.1:9090"  # Optional: Prometheus /metrics + /health
# trusted_proxies = ["10.0.0.0/8"]

[rate_limit]
per_ip_per_minute = 60
per_ip_burst = 10
```

Env var examples: `IFCONFIG_SERVER__BIND=0.0.0.0:8080`, `IFCONFIG_BASE_URL=ip.pdt.sh`.
Structured JSON logging: `IFCONFIG_LOG_FORMAT=json`.
Print effective config and exit: `--print-config` flag.

GeoIP data in `data/`: `GeoLite2-City.mmdb`, `GeoLite2-ASN.mmdb`.

## CI/CD

GitHub Actions: check → clippy → fmt → build/test → Docker integration tests. All CI jobs (except fmt) build the frontend before cargo operations. Pushing to `prod` branch auto-builds and pushes Docker image to GHCR (`ghcr.io/lukaspustina/ifconfig-rs:latest`).

## Common Patterns

- Routes and handlers are generated via declarative macros — follow existing macro invocations when adding new endpoints.
- `Ifconfig` struct in `backend/mod.rs` is the central data model — all endpoint responses derive from it. `Location` includes `accuracy_radius_km: Option<u16>` from GeoIP. `Network` struct holds IP classification (type, provider, flags).
- CLI client detection in `negotiate.rs` checks User-Agent patterns and `Accept: */*` header.
- Config values are loaded from a TOML file (`ifconfig.dev.toml` for local dev) via the `config` crate with env var overrides.
- `AppState` is shared via Axum's `State` extractor; all backends are `Arc`-wrapped.
- `build_app()` returns `AppBundle { app, admin_app }` — `admin_app` is `Some` only when `server.admin_bind` is configured and the metrics recorder installs successfully. In tests, multiple `build_app()` calls silently skip metrics (global recorder can only be set once).
- Rate limit middleware emits `X-RateLimit-Limit`, `X-RateLimit-Remaining` on all responses and `Retry-After` on 429s. `/health` and `/ready` are exempt.
- Frontend assets are embedded at compile time via `rust-embed` — `cargo build` requires `frontend/dist/` to exist.
