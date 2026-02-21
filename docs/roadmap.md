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

## P2 — Next Sprint

Meaningful gaps that affect API consumers and documentation quality.

### 4. OpenAPI: query parameters undocumented on most endpoints

**Source**: fixes.md
**File**: `src/routes.rs` — utoipa annotations

`?ip=`, `?fields=`, and `?dns=` are supported on every standard endpoint but are only documented
on a handful (`/`, `/all`, `/ip`, `/batch`). Code generators and the Scalar UI omit them for
`/host`, `/location`, `/isp`, `/network`, `/tcp`, `/user_agent`, `/headers`.

**Fix**: Extract a shared set of utoipa `params` for `ip_param`, `fields_param`, and `dns_param`
and apply them to every `#[utoipa::path]` annotation that dispatches via `dispatch_standard()`.

---

### 5. OpenAPI: `/meta` endpoint missing from spec

**Source**: fixes.md
**File**: `src/routes.rs`

The SPA calls `/meta` on startup to get `site_name` and `version`, but the endpoint has no
`#[utoipa::path]` annotation and does not appear in the OpenAPI spec or the Scalar UI.

**Fix**: Add a `#[utoipa::path]` annotation to `meta_handler` and include `MetaResponse` (or
`ProjectInfo`) as a response schema.

---

### 6. OpenAPI: rate-limit response headers undocumented

**Source**: fixes.md
**File**: `src/routes.rs` — utoipa annotations

`X-RateLimit-Limit`, `X-RateLimit-Remaining`, and `Retry-After` (on 429) are implemented but
absent from all operation specs. API consumers reading the spec have no way to discover them.

**Fix**: Add response header definitions to the affected operations, or add a shared description
in the spec's top-level `info.description`.

---

### 7. OpenAPI: no operation tags — Scalar UI is a flat unorganized list

**Source**: fixes.md
**File**: `src/routes.rs` — utoipa annotations

Every endpoint appears in one undifferentiated list in Scalar. Grouping by tag (e.g. "IP",
"Location", "Network", "User Agent", "Probes", "Batch") makes the interactive docs usable.

**Fix**: Add `tags = ["IP"]` etc. to each `#[utoipa::path]` and register the tag definitions
in `#[openapi(tags(...))]`.

---

### 8. No integration test for IPv6 clients

**Source**: fixes.md
**File**: `tests/ok_handlers.rs`

All integration tests use `remote_v4()`. There is no `remote_v6()` helper and no test for an
IPv6 client, including the case of an IPv6 client hitting `/ipv4` (should return 404) or an
IPv4 client hitting `/ipv6` — which is tested but only one direction.

**Fix**: Add a `remote_v6()` helper and tests: IPv6 client on `/`, `/ipv6`, and `/ipv4`
(expect 404).

---

### 9. No CORS preflight test

**Source**: fixes.md
**File**: `tests/ok_handlers.rs`

`CorsLayer` is the outermost layer but `OPTIONS` requests are never exercised in tests. A
regression in CORS config would not be caught.

**Fix**: Add an integration test that sends `OPTIONS / HTTP/1.1` with `Origin` and
`Access-Control-Request-Method` headers and asserts the correct CORS response headers.

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
