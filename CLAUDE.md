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

**ifconfig-rs** is a "what's my IP" web service written in Rust, powering **ip.pdt.sh**. Returns IP address, hostname, geolocation, ISP, and user agent info as plain text, JSON, or HTML depending on the client.

- **Author**: Lukas Pustina | **License**: MIT | **Edition**: 2021
- **Repository**: https://github.com/lukaspustina/ifconfig-rs

## Build & Test

```sh
cargo build                  # Build
cargo test --lib --no-fail-fast  # Unit tests (fast, no network)
cargo test                   # All tests including integration
cargo clippy                 # Lint
cargo fmt                    # Format
cargo run                    # Local dev server on :8000
make tests                   # Unit + Docker integration + Playwright E2E
make integration             # Docker-based integration tests only
make acceptance              # Playwright E2E tests only
make docker-build            # Production Docker image
```

### Test Guidelines

- `cargo test --lib` is the fast reliable check — no network or external services needed.
- `cargo test` also runs integration tests in `tests/ok_handlers.rs` (40+ tests covering all endpoints and content types) and `tests/error_handler.rs`.
- Docker integration tests (`make integration`) build and test inside a container via `tests/Dockerfile.tests`.
- Playwright E2E tests (`make acceptance`) run against production at `https://ip.pdt.sh` across Chromium, Firefox, and WebKit.

## Architecture

```
Request → Guards (extract IP, UA) → Fairing (X-Forwarded-For rewrite)
        → Routes (content negotiation by rank) → Handlers → Response (text/json/html)
```

Key modules:
- `src/lib.rs` — App bootstrap, `Config` struct, `rocket()` builder
- `src/backend/mod.rs` — Core logic: `get_ifconfig()` orchestrates GeoIP, reverse DNS, UA parsing
- `src/backend/user_agent.rs` — UA parsing wrapper around `uaparser`
- `src/routes.rs` — Macro-generated routes with ranked content negotiation (CLI → JSON → plain → HTML)
- `src/handlers.rs` — Macro-generated response formatters for each content type
- `src/guards.rs` — `RequesterInfo` (IP, UA, URI extraction) + `CliClientRequest` (curl/httpie/wget detection)
- `src/fairings.rs` — `XForwardedFor` middleware for load balancer environments + `SecurityHeaders` response fairing

**Content negotiation rank order**: CLI detection (rank 1) → JSON Accept header (rank 2) → plain text (rank 3) → HTML default (rank 4).

**API endpoints**: `/`, `/ip`, `/tcp`, `/host`, `/location`, `/isp`, `/user_agent` — all support `.json` suffix and `Accept` header negotiation.

## Configuration

Runtime config via `Rocket.toml`:
- **debug/release** — Standard local dev, uses TCP remote IP directly
- **xforwarded** — Production behind load balancer (Koyeb), reads `X-Forwarded-For` header

GeoIP data in `data/`: `GeoLite2-City.mmdb`, `GeoLite2-ASN.mmdb`.

## CI/CD

GitHub Actions: check → clippy → fmt → build/test → Docker integration tests. Pushing to `prod` branch auto-builds and pushes Docker image to GHCR (`ghcr.io/lukaspustina/ifconfig-rs:latest`).

## Common Patterns

- Routes and handlers are generated via declarative macros — follow existing macro invocations when adding new endpoints.
- `Ifconfig` struct in `backend/mod.rs` is the central data model — all endpoint responses derive from it.
- CLI client detection in `guards.rs` checks both User-Agent patterns and `Accept: */*` header.
- Config values are read from `Rocket.toml` via Rocket's managed state (`Config` struct in `lib.rs`).
