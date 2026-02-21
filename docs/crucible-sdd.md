# Software Design Document: ifconfig-rs Enrichment Evolution

**Status:** Phases 1a, 1b, 2, 3, 4 complete
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
| **L1 — Static lists** | Cloud provider CIDRs, Tor exits, Feodo C2, VPN ranges, datacenter ranges, bot ranges, Spamhaus DROP | Offline with data files, optional background refresh | <1us (HashSet/trie lookup) |
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
3. **Commit 3** — Make `get_ifconfig` and `make_ifconfig` async. Replace `dns-lookup` with async DNS for PTR lookups. Update `dispatch_standard` and `ip_version_dispatch` to async.

**DNS replacement — mhost:**

```toml
[dependencies]
mhost = { version = "0.11", default-features = false }
```

Uses mhost (same author) as the DNS library. mhost wraps `hickory-resolver` and provides multi-server concurrent DNS lookups with result aggregation. Initially blocked on a build bug in mhost 0.11.0 (`serde_json::Error` not feature-gated), which was worked around with `hickory-resolver` directly. mhost 0.11.1 fixed the build defect, and we migrated back. Note: `build_app()` and `AppState::new()` are now async because `ResolverGroupBuilder::build()` is async. We use `Resolver::lookup()` (single-resolver) rather than `ResolverGroup::lookup()` for PTR queries because the latter's `uni_lookup` path holds a non-Send `ThreadRng` across an await point, making the future incompatible with Axum's Send requirement.

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
| Async backend with mhost DNS (§3.2, commit 3) | `a5cce31` | +877/-44 | Done |

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

### Phase 2: The Differentiator ✓

*Features that create competitive separation. **COMPLETED** 2026-02-21.*

**Commit ordering** — each commit is independently testable with all tests passing:

| # | Commit | Est. LOC | Scope | Status |
|---|--------|----------|-------|--------|
| 1 | Data pipeline | Makefile | `data/Makefile` targets for `cloud_provider_ranges.jsonl`, `feodo_botnet_ips.txt`, `vpn_ranges.txt`. No Rust changes. See §4.1. | Done |
| 2 | EnrichmentContext + ArcSwap + SIGHUP | ~300 | Group `{geoip_city, geoip_asn, tor, dns_resolver, ua_parser}` into `EnrichmentContext` struct with `load(&Config)` constructor. Store behind `ArcSwap` in `AppState`. SIGHUP handler calls `load()`, validates, swaps. Pure refactor of existing backends — no new features, no behavioral change. Future backends automatically participate in hot-reload. | Done |
| 3 | Cloud provider fingerprinting | ~400 | Add `CloudProviderDb` to `EnrichmentContext`. `ip_network_table` dep for CIDR trie. JSONL parser for `cloud_provider_ranges.jsonl`. New `cloud` field on `Ifconfig` response. AWS + GCP + Cloudflare; Azure next. | Done |
| 4 | Feodo C2 botnet list | ~50 | Add `FeodoBotnetIps` to `EnrichmentContext`. `HashSet<IpAddr>` from `feodo_botnet_ips.txt`. Same file-parsing pattern as existing `TorExitNodes`. | Done |
| 5 | ASN heuristic module | ~100 | New `src/backend/asn_heuristic.rs`. Pure functions: `classify_asn(asn_org: &str) -> AsnClassification`. Lookup table mapping ASN name patterns to hosting/VPN tags. No state in `EnrichmentContext`. Unit tests for known patterns. | Done |
| 6 | VPN detection | ~200 | Add `VpnRanges` to `EnrichmentContext`. CIDR prefix matching via `ip_network_table` (same trie type as cloud). Falls back to ASN heuristic from commit 5. | Done |
| 7 | `is_tor` → `network` object | ~150 | **Breaking change.** Consolidate cloud (commit 3), Feodo (commit 4), ASN heuristic (commit 5), VPN (commit 6), and existing Tor into `Network` struct. Remove top-level `is_tor` from `Ifconfig`. Update frontend `types.ts`. Version bump to 0.5.0. | Done |
| 8 | docker-compose.yml | docs | Example deployment with geoipupdate sidecar + data volume. | Done |

**Milestone:** All 170+ tests pass after each commit. Version bumped to 0.5.0. Frontend updated for `Network` object. Data pipeline operational with AWS, GCP, Cloudflare, Oracle, Fastly, DigitalOcean, Linode, GitHub, Azure cloud providers, Feodo C2, VPN ranges, datacenter ranges, bot ranges, and Spamhaus DROP lists.

**Why this order:**

- **Commit 1 (data pipeline) first** — unblocks local testing for all subsequent commits. No Rust changes.
- **Commit 2 (EnrichmentContext + ArcSwap + SIGHUP) early** — establishes the reload pattern before adding new backends. The SIGHUP handler is trivial: call `EnrichmentContext::load()`, validate, swap. Each subsequent commit (3–6) extends `load()` as part of adding its backend — the handler itself never changes. No retrofit.
- **Commits 3–6 (new backends) in dependency order** — cloud first (introduces `ip_network_table` trie, highest differentiation value), Feodo next (trivial, same HashSet pattern), ASN heuristic (pure functions, no backend state), VPN last (depends on both `ip_network_table` from commit 3 and ASN heuristic from commit 5).
- **Commit 7 (breaking change) last among code changes** — all classification sources must exist before consolidating into the `network` object. Clear boundary for the 0.5.0 version bump.
- **Commit 8 (docs) anytime after commit 2** — but logically last since it references the full feature set.

#### 4.1 Data Pipeline

**Design principle:** ifconfig-rs never fetches external data. All data files are acquired by the `data/Makefile` and placed on disk. ifconfig-rs reads them at startup and hot-reloads on SIGHUP. This preserves the offline-first guarantee — the binary has zero outbound network dependencies.

**New data files:**

| File | Source | Format | Size | Update cadence |
|------|--------|--------|------|----------------|
| `cloud_provider_ranges.jsonl` | AWS + GCP + Cloudflare (Azure next) | Normalized JSONL | ~2MB | Weekly |
| `feodo_botnet_ips.txt` | [Feodo Tracker](https://feodotracker.abuse.ch/downloads/ipblocklist.txt) (abuse.ch) | Plain text, one IP/line, `#` comments | ~10KB | Daily |
| `vpn_ranges.txt` | [X4BNet/lists_vpn](https://github.com/X4BNet/lists_vpn) (v4+v6 merged) | Plain text, one CIDR/line | ~500KB | Daily |

**Cloud provider JSONL normalization:**

Raw provider JSONs have different schemas. The Makefile fetches each and normalizes via `jq` into a single JSONL file:

```jsonl
{"cidr":"3.2.34.0/26","provider":"aws","service":"EC2","region":"af-south-1"}
{"cidr":"35.186.0.0/16","provider":"gcp","service":"Google Cloud","region":"us-central1"}
{"cidr":"104.16.0.0/13","provider":"cloudflare","service":null,"region":null}
```

One file, one Rust parser. Each line is self-contained — easy to validate, diff, and debug.

**Cloud provider sources:**

| Provider | URL | Notes |
|----------|-----|-------|
| AWS | `https://ip-ranges.amazonaws.com/ip-ranges.json` | Stable URL. JSON with `service`, `region`, IPv4+IPv6. |
| GCP | `https://www.gstatic.com/ipranges/cloud.json` | Stable URL. JSON with `service`, `scope`. |
| Cloudflare | `https://www.cloudflare.com/ips-v4` + `ips-v6` | Stable URLs. Plain text CIDRs only — no service/region metadata. |
| Azure | *Next after initial three.* Scrape download link from `https://www.microsoft.com/en-us/download/details.aspx?id=56519`. Rotating URL, but standard ecosystem approach (same as Terraform Azure provider). | JSON with `serviceTags`, each containing `addressPrefixes` + `region`. |

Providers without official machine-readable CIDR lists (Hetzner, DigitalOcean, etc.) are handled by the ASN name heuristic in Rust code, not by data files.

**VPN data sources:**

X4BNet/lists_vpn is the initial source — it aggregates Mullvad, NordVPN, ExpressVPN, Surfshark, and others into consolidated `vpn-ipv4.txt` and `vpn-ipv6.txt` files, updated daily via GitHub Actions. These are merged into a single `vpn_ranges.txt` in the Makefile.

Additional VPN sources (individual provider relay lists, ASN-based prefix dumps) are deferred. The ASN name heuristic in `src/backend/asn_heuristic.rs` provides fallback coverage for VPN providers not in X4BNet.

**Updated `data/` directory:**

```
data/
├── Makefile                        # existing + new targets
├── Dockerfile                      # scratch image with all files
├── GeoLite2-City.mmdb              # existing (manual/geoipupdate)
├── GeoLite2-ASN.mmdb               # existing (manual/geoipupdate)
├── regexes.yaml                    # existing (uap-core)
├── tor_exit_nodes.txt              # existing (torproject)
├── feodo_botnet_ips.txt            # abuse.ch
├── cloud_provider_ranges.jsonl     # AWS+GCP+Cloudflare+Oracle+Fastly+DO+Linode+GitHub+Google Services+Azure
├── vpn_ranges.txt                  # X4BNet (v4+v6 merged)
├── datacenter_ranges.txt           # X4BNet datacenter list
├── bot_ranges.jsonl                # Googlebot+Bingbot+Applebot+GPTBot
└── spamhaus_drop.txt               # Spamhaus DROP+EDROP+DROPv6
```

#### 4.2 ASN Name Heuristic

A separate module `src/backend/asn_heuristic.rs` contains ASN-name-based classification rules. This provides:

- **Hosting/datacenter detection** for providers without official CIDR lists (Hetzner, DigitalOcean, OVH, Vultr, Linode, etc.) — matched by ASN organization name from the existing MaxMind ASN database.
- **VPN fallback** for VPN providers not covered by X4BNet — matched by known VPN ASN names.

The heuristic is a single point of change: a lookup table mapping ASN name patterns to classification tags. No external data dependency — uses the MaxMind ASN data already loaded.

```rust
// src/backend/asn_heuristic.rs — single source of truth for name-based classification
pub enum AsnClassification {
    Hosting { provider: &'static str },
    Vpn { provider: &'static str },
    None,
}

pub fn classify_asn(asn_org: &str) -> AsnClassification { ... }
```

#### 4.3 Cloud Provider Response Extension

```json
"cloud": {
  "provider": "AWS",
  "service": "EC2",
  "region": "eu-central-1"
}
```

Provider, service, and region populated from `cloud_provider_ranges.jsonl` via longest-prefix-match in `ip_network_table`. `null` fields when data is unavailable — never guess. Providers matched only by CIDR data have `"service": null, "region": null` (e.g., Cloudflare).

#### 4.4 Network Classification Response Extension

Replaces top-level `is_tor`:

```json
"network": {
  "type": "cloud",
  "provider": "AWS",
  "is_datacenter": true,
  "is_vpn": false,
  "is_tor": false,
  "is_proxy": false
}
```

Classification priority (highest wins for `type`): cloud → bot → VPN → Tor → botnet_c2 → threat → hosting → residential. Boolean flags are independent — an IP can be both `is_datacenter: true` and `is_vpn: true`.

The `network` object includes `is_bot` (true when IP matches known bot CIDR from Googlebot/Bingbot/Applebot/GPTBot) and `is_threat` (true when IP falls in a Spamhaus DROP/EDROP/DROPv6 range). Additional `is_datacenter` coverage comes from the X4BNet datacenter list alongside ASN heuristics.

This is a **breaking change** from the current API where `is_tor` is a top-level boolean on `Ifconfig`. The `network` object consolidates all IP classification into a single structure. Version bump to 0.5.0.

### Phase 3: Pipeline Integration ✓

*Features for SIEM/automation consumers. **COMPLETED** 2026-02-21.*

| # | Item | LOC | Status |
|---|------|-----|--------|
| 1 | `/ip/cidr` endpoint | ~15 | Done |
| 2 | `?ip=` arbitrary IP lookup | ~80 | Done |
| 3 | Field filtering (`?fields=`) | ~80 | Done |
| 4 | Batch endpoint (`POST /batch`) | ~300 | Done |
| 5 | OpenAPI spec via utoipa | ~250 | Done |

**Milestone:** All 209 tests pass (105 unit + 99 ok_handlers + 5 rate_limit). No breaking API changes from Phase 2.

**Implementation notes:**

- **`/ip/cidr`:** Returns `{ip}/32` (IPv4) or `{ip}/128` (IPv6) as plain text. Terraform/Ansible convenience endpoint.

- **`?ip=` arbitrary IP lookup:** All endpoints using `dispatch_standard()` gained an optional `?ip=` query parameter. When present, lookup targets that IP instead of the caller's. Input validation rejects RFC 1918, link-local, loopback, and unspecified addresses (400 Bad Request). PTR/reverse DNS is skipped by default for arbitrary IPs (opt-in via `?dns=true`). `IfconfigParam` gained `skip_dns: bool` to support this.

- **Field filtering (`?fields=`):** Parses `?fields=ip,location,isp` from the URI, filters the `serde_json::Value` after `to_json_fn()` produces it. Top-level field names only. Applies to JSON, YAML, TOML, CSV formats. Combines with `?ip=`: `GET /all/json?ip=8.8.8.8&fields=ip,location`.

- **Batch endpoint:** `POST /batch` accepts a JSON array of IP addresses (max configurable, default 100). Disabled by default (`batch.enabled = true` in config). N IPs consume N rate-limit tokens (checked before processing). Per-IP error handling: invalid/private IPs return `{"error": "...", "input": "..."}` inline. Full content negotiation: `/batch/json`, `/batch/yaml`, `/batch/toml`, `/batch/csv`. Batch CSV uses tabular format (one row per IP, dot-notated column headers). Exempt from standard per-request rate limiting middleware; batch handler applies its own N-token check via `check_key_n`.

- **OpenAPI via utoipa:** The macro-free handler architecture from Phase 1a made utoipa viable (decision record confirmed). All 16 public handlers annotated with `#[utoipa::path]`. Response types derive `ToSchema`: `Ifconfig`, `Ip`, `Tcp`, `Host`, `Location`, `Isp`, `Network`, `UserAgent`, `Browser`, `OS`, `Device`. Spec served at `GET /api-docs/openapi.json`. Swagger UI deferred (adds too many deps for marginal value).

### Phase 4: API Surface & Operational Polish ✓

*Expanding the data model with already-available MaxMind data, fixing a documented bug, and adding operational features. **COMPLETED** 2026-02-21.*

| # | Item | LOC | Status |
|---|------|-----|--------|
| 1 | Region/state, postal code, EU flag in Location | ~60 | Done |
| 2 | Fix trusted proxies CIDR parsing | ~65 | Done |
| 3 | X-GeoIP-Database-Date response header | ~35 | Done |
| 4 | Structured request logging via TraceLayer | ~2 | Done |
| 5 | `notify` filesystem watcher for auto-reload | ~120 | Done |

**Milestone:** All 213 tests pass (108 unit + 99 ok_handlers + 1 error_handler + 5 rate_limit). No breaking API changes.

**Implementation notes:**

- **Location expansion:** Added `region`, `region_code`, `postal_code`, `is_eu` to `Location` struct. Extracted from MaxMind GeoLite2-City data already present but not surfaced: `subdivisions[0].names.english`, `subdivisions[0].iso_code`, `postal.code`, `country.is_in_european_union`. Frontend InfoCards component shows region between city and country, and an EU badge on country rows.

- **Trusted proxies CIDR fix:** Config accepts `trusted_proxies = ["10.0.0.0/8"]` but `extractors.rs` only did `IpAddr::from_str()` which silently fails on CIDR strings. Moved CIDR parsing to `AppState::new()` using `ip_network::IpNetwork` (already a dependency). `extract_client_ip()` now accepts `&[IpNetwork]` and uses `.contains(ip)` for CIDR containment.

- **X-GeoIP-Database-Date:** `GeoIpCityDb::build_epoch()` reads `metadata.build_epoch` from the MMDB reader. `EnrichmentContext` stores the epoch at load time. A new `geoip_date_headers` middleware emits `X-GeoIP-Database-Date` (HTTP date format via `httpdate`) and `X-GeoIP-Database-Age-Days` (integer) on all responses. Operators can alert on stale databases.

- **TraceLayer:** Added `tower_http::trace::TraceLayer::new_for_http()` as outermost middleware layer. Logs HTTP method, URI, status code, and latency for every request. Works with both plain text and structured JSON logging (`IFCONFIG_LOG_FORMAT=json`).

- **Filesystem watcher:** Opt-in `watch_data_files = true` config option. Spawns a `notify::RecommendedWatcher` monitoring parent directories of all configured data files with `NonRecursive` mode. Events are debounced via 500ms tokio sleep + channel drain, then trigger `reload_enrichment()` — the same function used by SIGHUP. Handles atomic renames (geoipupdate pattern). Reload logic extracted from SIGHUP handler into shared `reload_enrichment()` async fn.

---

## 5. Rate Limiting Model

The current rate limiter (governor, keyed by IP) is extended with a clear scoping model:

| Scope | Behavior | Status |
|-------|----------|--------|
| **Main port (8080)** | All API endpoints rate-limited per IP. `/health` and `/ready` exempt. | **Done** |
| **Admin port (configurable)** | No rate limiter. Not publicly exposed — protected by network policy. Serves `/metrics` (Prometheus) and `/health`. | **Done** |
| **Batch endpoint** | A batch of N IPs costs N rate-limit tokens. Rate-limit check happens before processing. `/batch` exempt from standard middleware; handler applies its own N-token check via `check_key_n`. | **Done** |
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
| GeoIP Result Caching (moka) | Profile first. MaxMind lookups are sub-microsecond. DNS is the bottleneck and hickory-resolver (via mhost) handles DNS caching internally. |
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
| 1a | `mhost` 0.11 (default-features = false) | Async DNS (PTR lookups) replacing blocking `dns-lookup` | **Added** — wraps hickory-resolver with multi-server concurrent lookups. Initially used hickory-resolver directly due to mhost 0.11.0 build bug; migrated to mhost after 0.11.1 fix |
| 1b | `metrics-exporter-prometheus` 0.16 | Prometheus metrics recorder + `/metrics` render handle | **Added** — `axum-prometheus` was originally planned but panics on repeated global recorder installation (breaks integration tests) |
| 1b | `metrics-process` 2.3 | OS-level process metrics (CPU, memory, FDs) for `/metrics` | **Added** |
| 1b | `tracing-subscriber` json feature | Structured JSON log output via `IFCONFIG_LOG_FORMAT=json` | **Added** (feature flag on existing dep) |
| 2 | `arc-swap` 1 | Atomic pointer swap for `EnrichmentContext` hot-reload via SIGHUP | **Added** |
| 2 | `ip_network` 0.4 + `ip_network_table` 0.2 | IP prefix trie for cloud provider + VPN CIDR matching | **Added** |
| 2 | `regex` 1 | ASN name pattern matching in heuristic classifier | **Added** |
| 2 | `notify` 7 | Filesystem watcher for automatic hot-reload (complement to SIGHUP) | **Added** in Phase 4 — opt-in via `watch_data_files = true` |
| 4 | `httpdate` 1 | HTTP date formatting for X-GeoIP-Database-Date header | **Added** in Phase 4 — already transitive dep of hyper |
| 3 | `utoipa` 5 (features: `axum_extras`) | OpenAPI 3.1 spec generation from code annotations | **Added** — works cleanly with explicit handler functions from Phase 1a |

`dns-lookup` was removed in Phase 1a.

---

## 9. Open Questions

1. ~~**Cloud provider update cadence**~~ — Resolved: data fetching is external (`data/Makefile`). ifconfig-rs reads files at startup and hot-reloads on SIGHUP. Update cadence is controlled by however often the operator runs the data pipeline (cron, CI, geoipupdate sidecar).
2. ~~**VPN range data sources**~~ — Resolved: X4BNet/lists_vpn as primary source. ASN name heuristic in `src/backend/asn_heuristic.rs` as fallback. Additional sources deferred until X4BNet proves insufficient.
3. ~~**mhost version pinning**~~ — Resolved: migrated to mhost 0.11.1 which fixes the `serde_json::Error` feature-gating build defect from 0.11.0.
4. **Azure CIDR download URL:** Microsoft rotates the download URL for their IP ranges JSON. Plan: scrape the download link from the details page (`?id=56519`). Standard approach (used by Terraform Azure provider). Accepted risk: if page structure changes, the Makefile target fails and the last good file is kept.

---

## References

- [Original RFC](crucible-rfc.md)
- [mhost](https://github.com/lukaspustina/mhost) — Async DNS library (same author) wrapping hickory-resolver — used for PTR lookups
- [hickory-resolver](https://docs.rs/hickory-resolver) — Underlying async DNS resolver (transitive dep via mhost)
- [ip_network_table](https://docs.rs/ip_network_table) — IP prefix trie
- [metrics-exporter-prometheus](https://docs.rs/metrics-exporter-prometheus) — Prometheus metrics exporter (replaced axum-prometheus due to global recorder conflict in tests)
- [metrics-process](https://docs.rs/metrics-process) — OS-level process metrics
- [utoipa](https://docs.rs/utoipa-axum/latest/utoipa_axum/) — OpenAPI for Axum
- [Feodo Tracker](https://feodotracker.abuse.ch/) — Botnet C2 IP blocklist (abuse.ch)
- [X4BNet/lists_vpn](https://github.com/X4BNet/lists_vpn) — VPN provider IP ranges
- [AWS IP Ranges](https://ip-ranges.amazonaws.com/ip-ranges.json) — Official AWS IP range data (JSON)
- [GCP IP Ranges](https://www.gstatic.com/ipranges/cloud.json) — Official Google Cloud IP range data (JSON)
- [Cloudflare IP Ranges](https://www.cloudflare.com/ips/) — Official Cloudflare IP ranges (plain text)
- [Azure IP Ranges](https://www.microsoft.com/en-us/download/details.aspx?id=56519) — Official Azure IP range download page (rotating URL)
