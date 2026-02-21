# ifconfig-rs — Roadmap

> Consolidated open issues from `review.md` (code review, Feb 2026) and `fixes.md` (feature/UX review, Feb 2026).
> Already-resolved findings are excluded. Items are ordered by priority within each tier.

---

## P1 — Fix Soon ✅

### ~~1. Unknown format suffix returns 200 + SPA instead of 404~~ ✅

`negotiate()` now returns `NegotiatedFormat::Unknown` for unrecognized suffixes; both
`dispatch_standard()` and `ip_version_dispatch()` return 404 before any enrichment lookup.
Two unit tests added.

### ~~2. Network status badges are visually identical~~ ✅

Replaced `tor-badge tor` on all four flags with semantic classes: `net-badge--vpn` (blue),
`net-badge--tor` (amber), `net-badge--bot` (orange), `net-badge--threat` (red). All contrast
ratios verified ≥4.5:1 against dark card background.

### ~~3. `X-RateLimit-Limit` reports burst capacity, not the per-minute rate~~ ✅

`X-RateLimit-Limit` now reports `per_ip_per_minute`. `X-RateLimit-Reset` (Unix timestamp) added
to 429 responses. Rate-limit integration tests updated.

---

## P2 — Next Sprint ✅

Meaningful gaps that affect API consumers and documentation quality.

### ~~4. OpenAPI: query parameters undocumented on most endpoints~~ ✅

`?dns=` param added to all standard endpoints (`/ip`, `/tcp`, `/location`, `/isp`,
`/user_agent`, `/network`, `/ipv4`, `/ipv6`) that pass through `dispatch_standard()`.

---

### ~~5. OpenAPI: `/meta` endpoint missing from spec~~ ✅

Added `#[utoipa::path]` annotation to `meta_handler` with `ProjectInfo` response schema.
`meta_handler` added to `#[openapi(paths(...))]`; `crate::state::ProjectInfo` added to
`components(schemas(...))` with `#[derive(utoipa::ToSchema)]`.

---

### ~~6. OpenAPI: rate-limit response headers undocumented~~ ✅

Documented in `info.description` under a "Rate Limiting" section: `X-RateLimit-Limit`,
`X-RateLimit-Remaining` on all responses; `Retry-After`, `X-RateLimit-Reset` on 429s.
Also fixed batch 429 responses to use `per_ip_per_minute` (not `per_ip_burst`) and include
`X-RateLimit-Reset`, consistent with middleware behaviour.

---

### ~~7. OpenAPI: no operation tags — Scalar UI is a flat unorganized list~~ ✅

Ten tags defined and registered: IP, Location, ISP, Network, TCP, Host, User Agent, Headers,
Batch, Probes. All `#[utoipa::path]` annotations updated with `tag = "..."`.

---

### ~~8. No integration test for IPv6 clients~~ ✅

Added `send_request_v6()` helper that binds the server to `[::1]:0` and connects via IPv6
loopback. Three new tests: IPv6 client on `/json` (version=6), on `/ipv6/json` (200), on
`/ipv4/json` (404).

---

### ~~9. No CORS preflight test~~ ✅

Added `cors_preflight_returns_correct_headers` test: sends `OPTIONS /` with `Origin` and
`Access-Control-Request-Method` headers, asserts 200/204 response with
`Access-Control-Allow-Origin: *` and `Access-Control-Allow-Methods` present. Also updated
`CorsLayer` to explicitly set `allow_methods(Any)` and `allow_headers(Any)` for a complete
preflight response.

---

## P3 — Backlog

Lower urgency: either requires external data, adds incremental coverage, or is a minor
improvement to existing behavior.

### 10. No explicit `?dns=true` integration test

**Source**: fixes.md
**File**: `tests/ok_handlers.rs`

DNS opt-in for `?ip=` queries is only implicitly covered. A test with a loopback address
(`?ip=127.0.0.1`) hits the global-IP guard before DNS. A test with a real public IP requires
network access in CI.

**Fix**: Use a mock DNS resolver or assert behavior on the `skip_dns` flag at the unit level.
Alternatively, test `?dns=true` without a GeoIP DB (the response should still be structurally
valid, just with null location fields).

---

### 11. No admin port integration test

**Source**: fixes.md
**File**: `tests/`

The admin bind, `/metrics` output, and the new `admin_token` bearer auth are untested. A
regression in the auth middleware or Prometheus rendering would not be caught.

**Fix**: Add a test that calls `build_app()` with `admin_bind` configured, binds the admin
app to a random port, and asserts: (a) `/metrics` returns 200 with `text/plain`, (b) with
`admin_token` set, `/metrics` returns 401 without credentials and 200 with them.

---

### 12. No `filtered_headers` regex integration test

**Source**: fixes.md
**File**: `tests/ok_handlers.rs`

The `filtered_headers` config regex drops matching headers from `/headers` responses, but no
test verifies this. A broken `RegexSet` at startup panics (so the negative path is tested via
`state.rs` unit tests) but the positive filtering behavior is untested end-to-end.

**Fix**: Build an `AppState` with a `filtered_headers` regex, send a request with a
matching header, and assert it does not appear in the `/headers/json` response.

---

### 13. No env-var config override test

**Source**: fixes.md
**File**: `src/config.rs`

`IFCONFIG_*` env var overrides via the `config` crate are untested. A typo in the separator
(`__` vs `_`) would silently fail.

**Fix**: Add a unit test that sets `IFCONFIG_SERVER__BIND=0.0.0.0:9999` (or similar low-risk
field) via `std::env::set_var` and asserts `Config::load(None)` picks it up.

---

### 14. GeoIP database-age header tests require live DB

**Source**: review.md §3.5
**File**: `tests/ok_handlers.rs`

`X-GeoIP-Database-Date` and `X-GeoIP-Database-Age-Days` are emitted by `geoip_date_headers`
middleware but never asserted. Tests cannot load a real `.mmdb` in CI (licensed data).

**Fix**: Add a thin `AppState` builder that accepts a mock `geoip_city_build_epoch`, or gate
the test with `#[ignore]` and a comment pointing to the data-file setup docs.

---

### 15. Network classification not tested end-to-end

**Source**: review.md §3.6

Cloud/VPN/datacenter/bot/threat backends have unit tests but no integration test verifies
that these flags propagate into `/network` JSON for a known test IP.

**Fix**: Same constraint as §14 — requires data files. Consider a synthetic test where the
`AppState` is built with a minimal mock CIDR database containing a known test IP.

---

### 16. API Explorer lacks arrow-key navigation (WCAG tablist)

**Source**: review.md §4.7
**File**: `frontend/src/components/ApiExplorer.tsx:119–128`

Endpoint buttons form a tablist but do not respond to left/right arrow keys. WCAG 2.1 §4.1.3
recommends this pattern for keyboard users.

**Fix**: Add `onKeyDown` handlers to the endpoint tab buttons that move focus left/right
through the list using `document.querySelectorAll('.endpoint-tab')`.

---

### 17. Frontend component unit tests

**Source**: review.md §4.9

The API Explorer cache logic, clipboard handling, and `ThemeToggle` localStorage persistence
are non-trivial but have no isolated unit tests (Vitest / `@testing-library/solid`).

**Fix**: Bootstrap Vitest with `@testing-library/solid` and add tests for: cache hit/miss in
`ApiExplorer`, clipboard copy toggle, and theme persistence round-trip.

---

### 18. Rate-limit config validated at AppState construction, not config parse

**Source**: review.md §2.3
**File**: `src/state.rs`, `src/config.rs`

`per_ip_per_minute must be > 0` panics at `AppState::new()`, not at `Config::load()`.
The error location is slightly misleading (the wrong phase of startup).

**Fix**: Add `#[serde(try_from = "u32")]` or a `validate()` method called from `Config::load()`
so the error is surfaced during config deserialization.

---

## Future / Nice-to-Have

Not bugs or coverage gaps — features or polish that could add value but have no current urgency.

| Item | Notes |
|---|---|
| `X-RateLimit-Reset` header | Unix timestamp; pairs with P1 §3 |
| Nested `?fields=` dot-notation | e.g. `?fields=location.city,isp.asn` |
| IP lookup form in SPA | Frontend for `?ip=` queries |
| Batch lookup UI | Frontend for `POST /batch` |
| CSV export / download button | Backend supports it; SPA doesn't expose it |
| `ETag` / `Last-Modified` headers | Enables 304 Not Modified for repeat requests |
| Enrichment quality Prometheus gauges | Null-rate per field; helpful for DB freshness monitoring |
| ASN routing prefix in ISP data | BGP prefix alongside ASN org name |
| Data file acquisition docs | Where to get GeoLite2 DBs, `geoipupdate` setup |
| Embedded map in SPA | Currently links out to Google Maps |
