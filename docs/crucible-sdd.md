# Software Design Document: ifconfig-rs Enrichment Evolution

**Status:** Phase 1b complete
**Date:** 2026-02-21 (updated)
**Input:** [RFC crucible-rfc.md](crucible-rfc.md), multi-perspective design review, async migration analysis

---

## 1. Product Thesis

ifconfig-rs evolves from a "what's my IP" utility into a **self-hostable IP enrichment API**.

**Core value proposition:** Single binary with embedded frontend, zero external dependencies in the default configuration, all lookups sub-millisecond, self-hosted so IP queries never leave your infrastructure.

**Primary persona:** Platform/infrastructure engineer at a small-to-mid company (10-200 engineers) who currently uses a commercial IP enrichment API and wants to eliminate vendor lock-in, per-query pricing, or data sovereignty concerns.

**Competitive landscape:** Open-source alternatives exist (mpolden/echoip at 4.3K stars, cdown/geoip-http at 341K req/s) but are functionally frozen at `{ip, city, country, ASN}`. ifconfig-rs differentiates through:

- **Cloud provider fingerprinting** — no OSS competitor detects AWS/GCP/Azure/Cloudflare origin
- **Hosting-type classification** — residential vs. datacenter vs. VPN vs. Tor vs. proxy
- **Batch lookup** — `POST /batch` for up to 100 IPs, a feature only commercial APIs offer
- **Multi-format content negotiation** — JSON, YAML, TOML, CSV, plain text across all endpoints including batch
- **Single binary with embedded SPA** — zero deployment dependencies beyond the binary and MMDB files

Until Phase 2 ships, the competitive advantage is architectural: single Rust binary, sub-ms lookups, 5-format content negotiation, and an embedded SolidJS frontend for interactive exploration.

**The SolidJS frontend remains as a demo/marketing surface**, not a co-equal development priority.

---

## 2. Enrichment Layer Model

All features follow a strict layering that preserves the offline-first guarantee:

| Layer | Data Source | Availability | Latency |
|-------|-----------|--------------|---------|
| **L0 — Core** | GeoIP (MMDB), ASN, reverse DNS, UA parsing | Always (offline) | <1ms |
| **L1 — Static lists** | Cloud provider CIDRs, Tor exits, Feodo C2, VPN ranges | Offline with data files, optional background refresh | <1us (HashSet/trie lookup) |
| **L2 — External APIs** | RPKI, BGP, RDAP, GreyNoise, AbuseIPDB | Optional, online only | 50-2000ms |

Each layer degrades gracefully. Missing data produces `null` fields, never errors. API responses include an `enrichment_sources` array so consumers know what data quality to expect.

L2 features are **deferred** until the async enrichment pipeline exists and are always opt-in via configuration.

---

## 3. Architectural Prerequisites

The current codebase (~4K LOC, ~1.1K test LOC) is synchronous despite running on tokio. Two structural changes are required before enrichment features land.

### 3.1 Owned Data Types

**Problem:** `Ifconfig<'a>`, `Location<'a>`, `Isp<'a>` borrow `&'a str` from the MaxMind reader. This prevents caching results, holding data across `.await` points, and using `ArcSwap` for hot-reload.

**Solution:** Convert borrowed fields to owned `String`. The `'a` lifetime parameter is removed from all response structs.

| Struct | Borrowed fields | Allocation cost |
|--------|----------------|-----------------|
| `Location` | city, country, country_iso, timezone, continent, continent_code | ~100-200 bytes |
| `Isp` | name | ~20-60 bytes |
| `Ifconfig` | user_agent_header | Already owned in `RequesterInfo` |
| `Ip` | version | Was `&'a str`, converted to `String` |

Note: `Ip.version` was originally `&'a str` (always `"4"` or `"6"`). Using `&'static str` would have avoided allocation but conflicts with serde's `Deserialize` derive (the `'de` lifetime cannot satisfy `'static`). Converted to `String` — negligible cost for a 1-byte string.

**Status: DONE** (commit `c136ae6`). ~57 LOC changed across `backend/mod.rs` and `handlers.rs`. Pure refactor, zero behavioral change. All 117 tests pass unmodified.

### 3.2 Async Backend + Handler Flattening

**Problem:** `get_ifconfig()` in `backend/mod.rs` calls `dns_lookup::lookup_addr()` — a blocking libc `gethostbyaddr` call that stalls the tokio worker thread. The `handler!` and `endpoint_handler!` macros generate functions that each independently call `make_ifconfig()` → `get_ifconfig()`, computing the full `Ifconfig` struct multiple times per request path. The macros save ~15 lines per endpoint but make the code opaque, complicate debugging, and prevent future utoipa integration.

**Solution:** Three-commit migration (reordered from original four for lower risk — flattening macros *before* going async avoids async code generation inside macros):

1. **Commit 1** — Owned types (§3.1 above)
2. **Commit 2** — Flatten `handler!`, `endpoint_handler!`, `endpoint_format_handler!` macros into explicit modules with `to_json(&Ifconfig)` / `to_plain(&Ifconfig)` functions. Refactor `dispatch_standard` to compute `Ifconfig` once per request via fn pointers. Still synchronous.
3. **Commit 3** — Make `get_ifconfig` and `make_ifconfig` async. Replace `dns-lookup` with `hickory-resolver` for async PTR lookups. Update `dispatch_standard` and `ip_version_dispatch` to async.

**DNS replacement — hickory-resolver:**

```toml
[dependencies]
hickory-resolver = { version = "0.25", features = ["tokio", "system-config"] }
```

The original plan specified `mhost` (same author), but mhost 0.11.0 has a bug: its `Error` enum unconditionally references `serde_json::Error` even when `serde_json` is disabled via `default-features = false`, making the minimal build uncompilable. `hickory-resolver` is the underlying DNS library that mhost wraps, and provides a cleaner API for our use case: `TokioResolver::reverse_lookup(IpAddr)` does async PTR lookups directly, with built-in DNS caching and configurable timeouts. Since `TokioResolver::builder_tokio()` is synchronous, `build_app()` and `AppState::new()` remained sync — no changes to test infrastructure were needed.

**Implemented `dispatch_standard`:**

```rust
async fn dispatch_standard(
    format: NegotiatedFormat,
    req_info: &RequesterInfo,
    state: &AppState,
    to_json_fn: fn(&Ifconfig) -> Option<serde_json::Value>,
    to_plain_fn: fn(&Ifconfig) -> String,
) -> Response {
    if format == NegotiatedFormat::Html { return serve_spa(); }
    let ifconfig = handlers::make_ifconfig(/* backends, &state.dns_resolver */).await;
    match format {
        NegotiatedFormat::Plain => respond_plain(to_plain_fn(&ifconfig)),
        NegotiatedFormat::Json  => /* to_json_fn(&ifconfig) */,
        /* Yaml, Toml, Csv via to_json_fn + serialize_body */
    }
}
```

Each handler module (`handlers::ip`, `handlers::location`, etc.) exposes `to_json(&Ifconfig) -> Option<Value>` and `to_plain(&Ifconfig) -> String`. The dispatch layer computes `Ifconfig` once and passes fn pointers — no closures, no trait objects.

**Why flatten macros before async:** The original plan proposed async before macro flattening, but this would require generating async code inside macros (the `handler!` macro body calls `make_ifconfig` which becomes async). Reordering to flatten first, then go async, meant each step was independently testable with no async-in-macro complexity.

**Status: DONE** (commits `7f0929a` and `a5cce31`). Commit 2 changed +401/-388 LOC across `handlers.rs` and `routes.rs`. Commit 3 changed +877/-44 LOC (mostly `Cargo.lock` churn from the dependency swap). All 117 tests pass after each commit.

**Files unchanged:** `config.rs`, `lib.rs`, `main.rs`, `extractors.rs`, `middleware.rs`, `error.rs`, `negotiate.rs`, `format.rs`, `backend/user_agent.rs`, all test files. `state.rs` gained the `dns_resolver` field in Commit 3.

---

## 4. Phased Roadmap

### Phase 1a: Structural Refactoring ✓

*Prerequisite architecture work. No new features, no behavioral changes. **COMPLETED** 2026-02-21.*

| Item | Commit | LOC changed | Status |
|------|--------|-------------|--------|
| Owned data types (§3.1) | `c136ae6` | +57/-57 | Done |
| Flatten macros + compute-once dispatch (§3.2, commit 2) | `7f0929a` | +401/-388 | Done |
| Async backend with hickory-resolver (§3.2, commit 3) | `a5cce31` | +877/-44 | Done |

**Milestone:** All 117 existing tests pass after each commit. No new features, no API changes. The codebase is async-ready and macro-free.

### Phase 1b: Quick Wins ✓

*Low-risk features that build on the Phase 1a foundation. **COMPLETED** 2026-02-21.*

| Item | Commit | LOC changed | Status |
|------|--------|-------------|--------|
| GeoIP accuracy radius | `22316bf` | +10/-1 | Done |
| `/ready` readiness probe | `e47cc24` | +31/-3 | Done |
| Structured JSON logging (`IFCONFIG_LOG_FORMAT=json`) | `06455a8` | +24/-4 | Done |
| `--print-config` flag | `f9654fd` | +12/-5 | Done |
| Rate limit response headers | `d629d6d` | +126/-18 | Done |
| Admin port with Prometheus `/metrics` | `fc7ba9d` | +677/-12 | Done |

**Milestone:** All 120 tests pass (40 unit + 74 ok_handlers + 1 error_handler + 5 rate_limit). No breaking API changes.

**Implementation notes:**

- **GeoIP accuracy radius:** Added `accuracy_radius_km: Option<u16>` to `Location` struct. Populated from maxminddb's `geoip2::city::Location::accuracy_radius`. Appears in JSON/YAML/TOML/CSV automatically via serde, and in plain text output for `/all` and `/location`.

- **`/health` vs `/ready` semantics:** `/health` is a pure liveness probe — always returns `{"status": "ok"}` with 200. It does not check backends. `/ready` is a readiness probe — returns `{"status": "ready"}` (200) only when GeoIP City and ASN databases are loaded, otherwise `{"status": "not_ready", "reason": "..."}` (503). Both are exempt from rate limiting and cache headers.

- **Structured JSON logging:** `tracing-subscriber` gained the `json` feature. When `IFCONFIG_LOG_FORMAT=json` is set, log output switches to JSON lines. No new dependencies (tracing-serde is a transitive dep of the json feature).

- **`--print-config`:** Added `Serialize` derive to `Config`, `ServerConfig`, `RateLimitConfig`. The binary accepts `--print-config` alongside the config path; prints merged (file + env overrides) config as TOML and exits.

- **Rate limit response headers:** `KeyedRateLimiter` type changed to use `governor::middleware::StateInformationMiddleware`, which returns a `StateSnapshot` on success (exposing `remaining_burst_capacity()`). All rate-limited responses include `X-RateLimit-Limit` and `X-RateLimit-Remaining`. 429 responses additionally include `Retry-After` (seconds). Five integration tests cover success headers, 429 headers, and `/health`+`/ready` exemption.

- **Admin port:** Uses `metrics-exporter-prometheus` directly (not `axum-prometheus` — the latter panics on repeated global recorder installation, breaking integration tests that call `build_app()` multiple times per process). `build_app()` now returns `AppBundle { app, admin_app }` where `admin_app` is `Some` only when `server.admin_bind` is configured and the recorder installs successfully. `metrics-process` provides OS-level metrics (CPU, memory, FDs). Disabled by default.

### Phase 2: The Differentiator

*Features that create competitive separation. 1-2 months.*

| Item | LOC | Notes |
|------|-----|-------|
| Backend context struct | ~200 | Group backend references into `EnrichmentContext` struct. Introduced now because this phase adds new backends (cloud provider DB, threat lists). |
| Cloud/hosting provider fingerprinting | ~400 | IP prefix trie via `ip_network_table`. Load AWS, GCP, Azure, Cloudflare, Hetzner, DigitalOcean CIDRs at startup. Fetch directly from canonical provider URLs, not third-party aggregators. |
| Static threat lists: Tor periodic refresh + Feodo C2 | ~150 | Background task, hourly refresh. Same `HashSet<IpAddr>` pattern as existing Tor implementation. |
| VPN/proxy/hosting detection | ~200 | Known VPN IP range lists + hosting provider CIDRs + ASN name heuristic. |
| `is_tor` migration to `hosting` object | ~60 | Breaking change: remove top-level `is_tor`, add `hosting` object. Bump to 0.5.0. Update frontend `types.ts`. |
| GeoIP hot-reload via SIGHUP | ~100 | SIGHUP handler triggers reload. Validate new MMDB before swapping. On failure: log error, keep old DB. Add `notify`+`ArcSwap` later if demand warrants. |
| docker-compose.yml with geoipupdate sidecar | docs | Example deployment pattern, not a Helm chart. |

**Cloud provider response extension:**

```json
"cloud": {
  "provider": "AWS",
  "service": "EC2",
  "region": "eu-central-1"
}
```

Provider and region available for AWS and Cloudflare. Partial for GCP/Azure. `null` fields when data is unavailable — never guess.

**Hosting type response extension (replaces top-level `is_tor`):**

```json
"hosting": {
  "type": "cloud",
  "provider": "AWS",
  "is_datacenter": true,
  "is_vpn": false,
  "is_tor": false,
  "is_proxy": false
}
```

This is a **breaking change** from the current API where `is_tor` is a top-level boolean on `Ifconfig`. The `hosting` object consolidates all IP classification into a single structure. Version bump to 0.5.0.

### Phase 3: Pipeline Integration

*Features for SIEM/automation consumers. 1 quarter.*

| Item | LOC | Notes |
|------|-----|-------|
| Batch endpoint (`POST /batch`) | ~400 | Max 100 IPs. Rate-limit by IP count (N IPs = N tokens). Skip reverse DNS by default (opt-in via `?dns=true`). Reject RFC 1918/loopback. Disabled by default (`batch.enabled = true` in config). Full content negotiation: JSON, YAML, TOML, CSV output via Accept header or format suffix. |
| OpenAPI spec via utoipa | ~200 | Evaluate utoipa compatibility with the explicit handler functions from Phase 1a. If utoipa works: auto-generated spec from code annotations, served at `/api-docs/openapi.json`, Swagger UI at `/docs`. If utoipa doesn't fit: fall back to hand-written YAML (~300 LOC). |
| Field filtering (`?fields=ip,country,asn`) | ~60 | Filter `serde_json::Value` after serialization. Works for all endpoints uniformly, including batch. |
| `/ip/cidr` endpoint | ~10 | Returns `203.0.113.42/32`. Terraform/Ansible convenience. |

**Batch endpoint details:**

- Full content negotiation: JSON (default), YAML, TOML, CSV via Accept header or `POST /batch/csv` suffix
- CSV output is particularly useful: one row per IP, columns for fields — directly importable into spreadsheets and SIEM tools
- Opt-in via config (`batch.enabled = true`)
- Batch of N IPs costs N rate-limit tokens
- Input validation: reject RFC 1918, link-local, loopback (leaks internal topology)
- Per-IP error handling in response array, never global 500
- Reverse DNS skipped by default (expensive for batch), opt-in via `?dns=true`

**OpenAPI decision record:** The original SDD specified hand-written YAML because the `handler!` macro architecture conflicted with utoipa's proc macros. Phase 1a flattens these macros to explicit functions, which should make utoipa viable. Evaluate after Phase 1a ships. If utoipa annotations work cleanly on the new handler functions, use utoipa (auto-generated, won't drift). If not, hand-write the spec.

---

## 5. Rate Limiting Model

The current rate limiter (governor, keyed by IP) is extended with a clear scoping model:

| Scope | Behavior | Status |
|-------|----------|--------|
| **Main port (8080)** | All API endpoints rate-limited per IP. `/health` and `/ready` exempt. | **Done** |
| **Admin port (configurable)** | No rate limiter. Not publicly exposed — protected by network policy. Serves `/metrics` (Prometheus) and `/health`. | **Done** |
| **Batch endpoint** | A batch of N IPs costs N rate-limit tokens. Rate-limit check happens before processing. | Phase 3 |
| **Response headers** | `X-RateLimit-Limit`, `X-RateLimit-Remaining` on all rate-limited responses. `Retry-After` on 429 responses. | **Done** |

Note: `X-RateLimit-Reset` was dropped from the plan — governor's `StateInformationMiddleware` exposes `remaining_burst_capacity()` but not a reset timestamp. `Retry-After` (seconds-to-wait) on 429 responses serves the same purpose for clients that need backoff information.

---

## 6. Deferred

These features require an async enrichment pipeline with caching, timeouts, retry logic, and graceful degradation for external API failures. They are not scheduled but the architecture supports them once Phases 1-3 are complete.

| Feature | Reason for deferral |
|---------|-------------------|
| RPKI Validity | Wrong audience — network operators, not the target persona. External API dependency (RIPEstat) or heavy sidecar (Routinator). |
| BGP Routing Data | Marginal value over existing MaxMind ASN data. External API dependency. |
| Abuse Contact Lookup (RDAP) | Five RIR endpoints with different schemas. Rarely needed in hot path. |
| GreyNoise / AbuseIPDB | Rate-limited free tiers (10-1000 queries/day) make them non-viable for production. Breaks offline-first positioning. |
| OpenTelemetry Tracing | Valuable but not blocking. Add when operational maturity demands it. |
| GeoIP Result Caching (moka) | Profile first. MaxMind lookups are sub-microsecond. DNS is the bottleneck and hickory-resolver handles DNS caching internally. |
| Layered config with clap | Current config-file + env-var approach works. clap adds value only with complex subcommands. |

---

## 7. Killed

| Feature | Reason |
|---------|--------|
| DNS server (hickory-dns) | Running a DNS server on port 53 in containers requires root/CAP_NET_BIND_SERVICE. ~50 transitive deps. Different product. |
| gRPC (tonic) | Adds protoc build dependency. Users use curl, not protobuf. |
| STUN NAT-Type | Requires multiple public IPs per RFC 5389 — impossible as a side-feature. |
| HTTP/3 (quinn) | Let the reverse proxy handle QUIC. Zero user-visible benefit for <2KB responses. |
| JSONP | CORS exists. |
| SSE IP-Change Monitoring | A cron job hitting `/ip` is strictly better for this use case. |
| Helm Chart | Maintenance trap for a solo maintainer. Provide example YAML manifests and docker-compose instead. |
| Admin API (separate endpoints) | "Restart the pod" is the admin API for a stateless service with sub-second startup. SIGHUP covers GeoIP reload. Prometheus metrics on admin port is sufficient operational surface. |

---

## 8. New Dependencies

| Phase | Crate | Purpose | Status |
|-------|-------|---------|--------|
| 1a | `hickory-resolver` 0.25 (tokio, system-config) | Async DNS (PTR lookups) replacing blocking `dns-lookup` | **Added** — `mhost` was originally planned but has a build bug with `default-features = false` |
| 1b | `metrics-exporter-prometheus` 0.16 | Prometheus metrics recorder + `/metrics` render handle | **Added** — `axum-prometheus` was originally planned but panics on repeated global recorder installation (breaks integration tests) |
| 1b | `metrics-process` 2.3 | OS-level process metrics (CPU, memory, FDs) for `/metrics` | **Added** |
| 1b | `tracing-subscriber` json feature | Structured JSON log output via `IFCONFIG_LOG_FORMAT=json` | **Added** (feature flag on existing dep) |
| 2 | `ip_network_table` | IP prefix trie for cloud provider fingerprinting | Planned |
| 2 | `arc-swap` | Atomic pointer swap for hot-reload (when upgrading from SIGHUP) | Planned |
| 2 | `notify` | Filesystem watcher for hot-reload (optional, after SIGHUP) | Deferred — SIGHUP first |
| 3 | `utoipa` + `utoipa-axum` | OpenAPI spec generation (if compatible with handler functions) | Evaluate after Phase 1a (now eligible) |

`dns-lookup` was removed in Phase 1a.

---

## 9. Open Questions

1. **Cloud provider update cadence:** AWS/GCP/Azure publish IP ranges with weekly-ish changes. Daily refresh at startup sufficient, or periodic background refresh needed?
2. **VPN range data sources:** X4BNet/lists_vpn is one option. Are there more reliable/comprehensive public sources?
3. ~~**mhost version pinning**~~ — Resolved: switched to `hickory-resolver` 0.25 directly. mhost 0.11.0 has a build defect (`serde_json::Error` in `Error` enum is not feature-gated, making `default-features = false` uncompilable). `hickory-resolver` provides `TokioResolver::reverse_lookup(IpAddr)` with less indirection.

---

## References

- [Original RFC](crucible-rfc.md)
- [hickory-resolver](https://docs.rs/hickory-resolver) — Async DNS resolver (used for PTR lookups)
- [mhost](https://github.com/lukaspustina/mhost) — Async DNS library (same author) — originally planned but has build defect in 0.11.0
- [ip_network_table](https://docs.rs/ip_network_table) — IP prefix trie
- [metrics-exporter-prometheus](https://docs.rs/metrics-exporter-prometheus) — Prometheus metrics exporter (replaced axum-prometheus due to global recorder conflict in tests)
- [metrics-process](https://docs.rs/metrics-process) — OS-level process metrics
- [utoipa](https://docs.rs/utoipa-axum/latest/utoipa_axum/) — OpenAPI for Axum
- [Feodo Tracker](https://feodotracker.abuse.ch/) — Botnet C2 IP blocklist (abuse.ch)
- [X4BNet/lists_vpn](https://github.com/X4BNet/lists_vpn) — VPN provider IP ranges
