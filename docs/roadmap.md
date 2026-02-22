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

## P3 — Backlog ✅

Lower urgency: either requires external data, adds incremental coverage, or is a minor
improvement to existing behavior.

### ~~10. No explicit `?dns=true` integration test~~ ✅

Added `ip_param_dns_true_returns_valid_response` in `tests/ok_handlers.rs`: sends
`GET /all/json?ip=8.8.8.8&dns=true`, asserts 200 and that `host` is null or an object
(structurally valid regardless of whether DNS resolves).

---

### ~~11. No admin port integration test~~ ✅

Added `tests/admin.rs` with `admin_port_metrics_and_bearer_auth`: calls `build_app()` with
`admin_bind` and `admin_token` configured, binds the admin app, and asserts `/metrics` returns
401 without credentials, 401 with wrong token, and 200 with correct token (Prometheus text
body). Also asserts `/health` behaves identically. Gracefully skips if the metrics recorder
is already installed (process-level singleton).

---

### ~~12. No `filtered_headers` regex integration test~~ ✅

Added `filtered_headers_excluded_from_response` in `tests/ok_handlers.rs`: builds a custom
app with `filtered_headers = ["(?i)^x-test-secret$"]`, sends a request with that header,
and asserts the header does not appear in the `/headers/json` response body.

---

### ~~13. No env-var config override test~~ ✅

Added `env_var_overrides_top_level_field` and `env_var_overrides_nested_field_with_double_underscore`
in `src/config.rs` tests. Also fixed `Config::load()` to call `.prefix_separator("_")` so
that `IFCONFIG_SERVER__BIND` correctly maps to `server.bind` (config 0.15 strips the prefix
exactly, leaving a leading `_` without the separator option). Tests share `ENV_LOCK` mutex to
prevent concurrent interference; all tests calling `Config::load(None)` now acquire it.

---

### ~~14. GeoIP database-age header tests require live DB~~ ✅

Added `geoip_date_headers_present_when_db_loaded` in `tests/ok_handlers.rs` gated with
`#[ignore = "requires GeoIP database files in data/"]`. Documents expected behavior and can
be run manually with `cargo test -- --ignored`.

---

### ~~15. Network classification not tested end-to-end~~ ✅

Added `network_classification_propagates_to_json` in `tests/ok_handlers.rs` gated with
`#[ignore = "requires network classification data files in data/"]`. Same rationale as §14.

---

### ~~16. API Explorer lacks arrow-key navigation (WCAG tablist)~~ ✅

Endpoint tab buttons now have `role="tab"`, `aria-selected`, `tabIndex` (0 for active, -1
for others), and `onKeyDown` handlers for ArrowLeft/ArrowRight focus movement. The container
div has `role="tablist"` and `aria-label="API endpoints"`. Covered by the Vitest arrow-key
navigation test (item 17).

---

### ~~17. Frontend component unit tests~~ ✅

Bootstrapped Vitest 4 with `@solidjs/testing-library`, `@testing-library/jest-dom`, and
`happy-dom`. Added `frontend/vitest.config.ts` and `frontend/src/test-setup.ts` (with
Map-backed localStorage mock for happy-dom compatibility). Nine tests across two files:

- `ThemeToggle.test.tsx` (4 tests): reads theme from localStorage on mount, cycles
  dark→light→system, persists to localStorage, applies `data-theme` to `documentElement`.
- `ApiExplorer.test.tsx` (5 tests): renders collapsed, expands on click, cache hit/miss
  (waits for non-loading pre to ensure cache is populated), clipboard copy toggle,
  arrow-key navigation.

---

### ~~18. Rate-limit config validated at AppState construction, not config parse~~ ✅

Added `Config::validate()` (called from `Config::load()`) that returns
`Err(ConfigError::Message(...))` if `per_ip_per_minute == 0` or `per_ip_burst == 0`. The
panic in `AppState::new()` is now unreachable for these fields. Two unit tests added.

---

## Future / Nice-to-Have

Not bugs or coverage gaps — features or polish that could add value but have no current urgency.
Items marked ✏️ have been explored and have a concrete implementation plan below.

| Item | Notes |
|---|---|
| ~~`X-RateLimit-Reset` header~~ | Already implemented in P1 §3 |
| Nested `?fields=` dot-notation | e.g. `?fields=location.city,isp.asn` |
| IP lookup form in SPA ✏️ | Frontend for `?ip=` queries — plan below |
| Batch lookup UI | Frontend for `POST /batch` |
| CSV export / download button | Backend supports it; SPA doesn't expose it |
| `ETag` / `Last-Modified` headers | Enables 304 Not Modified for repeat requests |
| Enrichment quality Prometheus gauges | Null-rate per field; helpful for DB freshness monitoring |
| ASN routing prefix in ISP data ✏️ | BGP prefix alongside ASN org name — plan below |
| GeoIP unused fields ✏️ | Several free fields available in loaded DBs — analysis below |
| Data file acquisition docs | Where to get GeoLite2 DBs, `geoipupdate` setup |
| Embedded map in SPA | Currently links out to Google Maps |

---

## Implementation Notes

### IP Lookup Form in SPA

**No backend changes needed.** `GET /all/json?ip=<addr>` already exists, returns the full `Ifconfig` shape, and enforces global-IP validation (400 for private/loopback, silent fallback for unparseable strings).

**Signal architecture (`App.tsx`):**
- Add `lookedUpIp: Signal<string>` and `lookupError: Signal<string | null>`
- Extend `loadData(ip?: string)`: when `ip` provided hits `/all/json?ip=<ip>`, otherwise `/json` as today
- Add `handleLookup(ip)` and `handleReset()` callbacks

**New component `IpLookupForm.tsx`**, placed between `<IpDisplay>` and `<InfoCards>`:
- Props: `onLookup`, `onReset`, `currentIp`, `loading`, `error`
- Native `<form onSubmit>` so Enter-key works; `<label>` + `<input>` + submit + conditional "Back to my IP" reset button
- Input: `autocorrect="off"`, `autocapitalize="none"`, `spellcheck="false"`, `inputmode="text"` (numeric hides `.` on mobile)
- `role="alert"` error div; `aria-invalid` + `aria-describedby` on the input

**Client-side validation is critical:** invalid IP strings silently fall back to the caller's IP on the backend — the frontend must validate before submitting. Checks: parseable IPv4/IPv6, then reject RFC 1918 / loopback / ULA / link-local (mirrors `is_global_ip()`).

**`api.ts`:** add `fetchIfconfigForIp(ip)` hitting `/all/json?ip=<encoded>` — parses backend JSON error body on non-OK responses.

**`global.css`:** new form styles reusing existing tokens (`--accent`, `--error`, `--border`, `--font-mono`).

**Implementation order:**
1. `api.ts` — add `fetchIfconfigForIp`
2. `global.css` — add form CSS
3. `IpLookupForm.tsx` — new component with `validateIp` helper
4. `App.tsx` — wire signals and callbacks
5. `IpDisplay.tsx` — optional `lookedUpFrom?: string` prop for mode indicator
6. `IpLookupForm.test.tsx` — Vitest tests

---

### ASN Routing Prefix in ISP Data

**The prefix is already computed — it's just discarded.** `maxminddb`'s `LookupResult::network()` returns the BGP routing prefix as `ipnetwork::IpNetwork` (a transitive dep). The current `GeoIpAsnDb::lookup` drops the `LookupResult` after decoding the record body.

**Type bridging:** call `.to_string()` on `ipnetwork::IpNetwork` immediately inside `GeoIpAsnDb::lookup` — the prefix exits as a `String`. No new direct dependency needed.

**Changes required (no new deps):**

| File | Change |
|---|---|
| `src/backend/mod.rs:63–72` | Change `lookup` to return `Option<(geoip2::Isp<'_>, Option<String>)>` — keep `LookupResult`, call `.network().map(\|n\| n.to_string())` |
| `src/backend/mod.rs:155–170` | Add `prefix: Option<String>` to `Isp` struct + `#[schema(example = "203.0.113.0/24")]`; update `Isp::unknown()` |
| `src/backend/mod.rs:289–296` | Populate `isp.prefix` in `get_ifconfig()` |
| `src/handlers.rs:104–111` | `isp::to_plain` — new format: `"Example Telecom (AS64496, 203.0.113.0/24)\n"` with all four `(asn, prefix)` combinations |
| `src/handlers.rs:223–228` | `all::to_plain` — add `"prefix:     {}\n"` line after ASN |
| `src/handlers.rs` tests | Update `make_ifconfig` fixture and `isp_to_plain_with_asn` assertion |
| `frontend/src/lib/types.ts:61–64` | Add `prefix: string \| null` to `Isp` interface |
| `frontend/src/components/InfoCards.tsx:206–218` | Add `<Show when={isp().prefix != null}>` row with `class="card-value mono"` |

`routes.rs`, `Cargo.toml`, and all other files are unaffected. The batch endpoint and all format handlers inherit the field automatically through `Ifconfig.isp`.

---

### Unused GeoIP Fields

Both the City and ISP databases are only ~50% utilised. Fields available in the **currently loaded free databases**:

**GeoLite2-City (high value, zero new deps):**

| Field | Struct | Practical use |
|---|---|---|
| `registered_country` | City | Country where IP block was registered — differs from location for VPNs; strong proxy/VPN signal |
| `represented_country` + `type` | City | Embassy/military base detection; `type` values include `"military"`, `"diplomatic"` |
| `geoname_id` | Country, City, Continent, Subdivision | External links to GeoNames, Wikipedia, OpenStreetMap |
| `subdivisions[1..]` | City | County/district level below state — only first subdivision used today |
| Non-English `names` | Names | Localised names in DE, ES, FR, JA, PT-BR, RU, ZH-CN |

**GeoLite2-ISP / GeoIP2-ISP (already loaded):**

| Field | Tier | Practical use |
|---|---|---|
| `isp` | Paid | ISP brand name (distinct from ASN org — e.g. org=`"Google LLC"`, isp=`"Google Fiber"`) |
| `organization` | Paid | Organisation name for the IP block |
| `mobile_country_code` | Paid | Carrier identification for mobile IPs |
| `mobile_network_code` | Paid | Mobile network within country |

**Skip:** `metro_code` (deprecated by MaxMind), `DensityIncome` (out of scope), `AnonymousIp` DB (redundant with existing custom CIDR files), confidence scores (Enterprise-only paid DB).

**Recommended first additions:** `registered_country` (free, high signal for VPN detection) and `geoname_id` on country/city (free, enables rich external linking in the SPA).
