# Roadmap — ifconfig-rs

## Bugs

#### B1. ~~Hostname missing under IP display~~ (fixed Sprint 3)

`IpDisplay.tsx` still referenced the removed `host.name` field (pre-Sprint 1 struct) instead of `ip.hostname`. The hostname row was never shown and copy was broken.

#### B2. ~~Auto dark/light mode no longer follows system preference~~ (fixed)

Root cause: the inline script in `index.html` that applied the theme was blocked by CSP (`script-src 'self'` forbids inline scripts), so `data-theme` was never set and the page always rendered dark. Fixed by removing the inline script and using `@media (prefers-color-scheme: light) { :root:not([data-theme="dark"]) { ... } }` in CSS — system mode now requires zero JS. `ThemeToggle` now calls `removeAttribute("data-theme")` for system mode.

#### B3. ~~API Explorer responses go stale after a lookup~~ (fixed)

Root cause: component-level `Map` cache in `ApiExplorer.tsx` never expired and was never invalidated. Fixed by removing the map — the browser HTTP cache (`Cache-Control: private, max-age=60` + ETags) handles deduplication correctly.

#### B4. ~~API Explorer not updated on IP lookup~~ (fixed)

When a `?ip=` lookup was performed (e.g. 8.8.8.8), the API Explorer still showed the caller's own IP data and the curl command still read `curl host/json` (no `?ip=` param). Fixed by adding a `lookupIp: string | null` prop to `ApiExplorer`; when set, `?ip=<ip>` is appended to all fetch URLs and to the displayed curl command. The `createEffect` re-runs whenever `lookupIp` changes.

---

## Sprint 3 — Frontend: Core UX

#### 45. IP lookup form in SPA

Frontend form for `?ip=` queries — look up any public IP without leaving the page. No backend changes needed. Client-side validation is critical: invalid IPs silently fall back to the caller's IP on the backend. See Implementation Notes and discuss first.

#### 30. Skeleton loading screens

Replace the "Loading..." spinner with animated placeholder shapes matching the card layout.

#### 34. Copy UX improvements

Toast notifications instead of button state changes on copy. Make the full curl command in the API Explorer selectable and copyable as one action.

---

## Sprint 4 — Infrastructure & Quality

#### 37. ~~Add E2E tests to CI~~ (done)

New `e2e-test` job in CI: builds the production Docker image (which includes data from `ifconfig-rs-data`), starts the server on port 8000, installs Playwright, and runs the full acceptance suite. Playwright report uploaded as an artifact on failure.

#### 38. ~~Dependency vulnerability scanning~~ (done)

New `audit` job in CI: installs `cargo-audit` and runs `cargo audit`, then runs `npm audit --audit-level=high` for both `frontend/` and `tests/e2e/`.

#### 49. ~~Enrichment quality Prometheus counters~~ (done)

`ifconfig_null_field_total{field}` counter incremented in `get_ifconfig()` whenever `hostname`, `city`, `country`, `asn`, `org`, or `user_agent` is null. Use `rate(ifconfig_null_field_total[5m]) / rate(http_requests_total[5m])` to compute null rates per field in dashboards.

#### 52. ~~Data file acquisition docs~~ (done)

`data/README.md` documents all data sources with download instructions. `data/geoipupdate.conf.example` is the MaxMind config template (copy to `data/.geoip.conf` and fill in credentials). `make -C data get_all` (also `make update-data` from root) fetches everything. `Readme.md` and `CLAUDE.md` point here.

---

## Sprint 5 — Advanced Backend

#### 43. ~~Internal mode~~ (done)

A config flag (`internal_mode = true`) that allows the service to respond to requests from private and reserved IP ranges. Affected subnets: RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), loopback (`127.0.0.0/8`, `::1/128`), link-local (`169.254.0.0/16`, `fe80::/10`), ULA (`fc00::/7`). GeoIP lookups return no results (expected); `network.classification.type` reflects `"internal"`. `is_global_ip()` moved from `routes.rs` to `backend/mod.rs` (`pub fn`); `get_ifconfig()` short-circuits for non-global IPs returning the `"internal"` type without running classification.

#### 23. ~~DNS reverse lookup caching~~ (done)

Reverse DNS lookup has no caching. Add an in-memory LRU cache with a short TTL (e.g., 60s). Implemented: `DnsCache` type alias (`Mutex<LruCache<IpAddr, (Option<String>, Instant)>>`) in `backend/mod.rs`, shared via `Arc<DnsCache>` on `AppState`. Capacity 1024, TTL 60s. Both successful and failed lookups are cached. Mutex not held across any await point.

#### 22. Port reachability check — pending discussion

New endpoint `/port/{number}` that attempts a TCP connect back to the requester's IP. Returns open/closed/filtered. Needs a timeout and connection limits to avoid abuse. To be discussed after 43 and 23 are done.

---

## Sprint 6 — Richer Frontend

#### 29. Map visualization

Wont do: Lat/long are returned and already linked to Google Maps. Replace the external link with an embedded lightweight map (e.g., Leaflet + OpenStreetMap tiles).

#### 46. Batch lookup UI

Frontend interface for `POST /batch`. Paste a list of IPs, view results in a table. The batch endpoint already exists. This has to be discussed first, before it gets implemented, because I dont know if I want it.

#### 47. CSV export / download button

The backend already serves `/all/csv`. Add a download button in the SPA to export the current result. This has to be discussed first, before it gets implemented, because I dont know if I want it.

#### 33. WebRTC leak detection

Frontend-only: detect IPs leaked via WebRTC when using a VPN. Strong differentiator for privacy-conscious users. This has to be discussed first, before it gets implemented, because I dont know if I want it.

---

## Backlog

> Lower priority or higher effort. No current sprint assignment.

#### 53. Populate `network.classification.is_proxy`

`is_proxy` is currently always `false` — no proxy detection data source is wired up. Options: integrate a proxy IP list (similar to `vpn_ranges.txt`), use a third-party API, or derive from existing heuristics (e.g. known open-proxy ASNs). Needs a data source decision before implementation.

#### 21. IP decimal representation

Add `ip_decimal` to the `Ip` struct — the integer representation (e.g., `1.2.3.4` = `16909060`). This has to be discussed first, before it gets implemented, because I dont know if I want it.

#### 24. Multi-language location names

MaxMind already contains localized names in ~8 languages. A `?lang=de` query parameter — no new data sources needed. I like this.

#### 25. Whois lookup

Return WHOIS registration data for the IP's network block. Requires calling WHOIS servers or integrating a library. mhost has this functionality but it's a heavy dependency. This has to be discussed first, before it gets implemented, because I dont know if I want it.

#### 39. Code coverage reporting

Add tarpaulin (Rust) and c8 (JS) with reporting to Codecov or similar.

---

## Implementation Notes

### IP Lookup Form in SPA

**No backend changes needed.** `GET /all/json?ip=<addr>` already exists, returns the full `Ifconfig` shape, and enforces global-IP validation (400 for private/loopback).

**Signal architecture (`App.tsx`):**

- Add `lookedUpIp: Signal<string>` and `lookupError: Signal<string | null>`
- Extend `loadData(ip?: string)`: when `ip` provided hits `/all/json?ip=<ip>`, otherwise `/json` as today
- Add `handleLookup(ip)` and `handleReset()` callbacks

**New component `IpLookupForm.tsx`**, placed between `<IpDisplay>` and `<InfoCards>`:

- Props: `onLookup`, `onReset`, `currentIp`, `loading`, `error`
- Native `<form onSubmit>` so Enter-key works; `<label>` + `<input>` + submit + conditional "Back to my IP" reset button
- Input: `autocorrect="off"`, `autocapitalize="none"`, `spellcheck="false"`, `inputmode="text"`
- `role="alert"` error div; `aria-invalid` + `aria-describedby` on the input

**Client-side validation:** reject RFC 1918 / loopback / ULA / link-local (mirrors `is_global_ip()`).

**`api.ts`:** add `fetchIfconfigForIp(ip)` hitting `/all/json?ip=<encoded>` — parses backend JSON error body on non-OK responses.

**Implementation order:**

1. `api.ts` — add `fetchIfconfigForIp`
2. `global.css` — add form CSS
3. `IpLookupForm.tsx` — new component with `validateIp` helper
4. `App.tsx` — wire signals and callbacks
5. `IpDisplay.tsx` — optional `lookedUpFrom?: string` prop for mode indicator
6. `IpLookupForm.test.tsx` — Vitest tests

---

### Unused GeoIP Fields

**GeoLite2-City (free, remaining value):**

| Field | Practical use |
|---|---|
| `geoname_id` on country/city | External links to GeoNames, Wikipedia, OpenStreetMap |
| `represented_country` + `type` | Embassy/military detection (`"military"`, `"diplomatic"`) |
| `subdivisions[1..]` | County/district below state |
| Non-English `names` | Localised names in DE, ES, FR, JA, PT-BR, RU, ZH-CN |

**GeoLite2-ISP (paid tier only):** `isp` brand name, `organization`, `mobile_country_code`, `mobile_network_code`.

**Skip:** `metro_code` (deprecated), `AnonymousIp` DB (redundant with existing CIDR files), confidence scores (Enterprise-only).

---

## Won't Do

### 44. Nested `?fields=` dot-notation

Complexity cost (recursive descent in `filter_fields()`, special-casing in plain-text and CSV serialisers) outweighs the benefit given that `jq` covers the use case on the client side.

### 12. JSONP callback support

JSONP is obsoleted by CORS. Maintaining a JSONP code path adds complexity for a shrinking audience.

### 13. Prometheus metrics on the main port

The admin port (`server.admin_bind`) already exposes `/metrics` for those who need it.

---

## Done

### Sprint 1 — API v2: Data Model Cleanup (v0.9.0)

> Breaking changes shipped as a single version bump.

#### 42. Move `user_agent_header` into `user_agent.raw`

`user_agent_header` moved to `user_agent.raw` (nullable string on the UA object). When no UA is present the entire `user_agent` object remains null.

#### 41. Move `host` into `ip`

`host.name` flattened to `ip.hostname`. `Host` struct and top-level `host` key removed. `/host` endpoint removed.

#### 40. Merge `isp` into `network`, add `prefix`

`Isp` struct removed. `network` gains `asn`, `org`, `prefix`. `/isp` endpoint removed. `network` is now always present (non-optional).

#### Classification sub-object

`type` and all `is_*` flags moved from `Network` into a nested `network.classification` object.

Current API shape (v0.9.0):

```json
{
  "ip": { "addr": "5.63.60.1", "version": "4", "hostname": "5-63-60-1.example.net" },
  "tcp": { "port": 4810 },
  "location": {
    "city": "Berlin", "region": "Berlin", "region_code": "BE",
    "country": "Germany", "country_iso": "DE", "postal_code": "10115",
    "is_eu": true, "latitude": 52.52, "longitude": 13.405,
    "timezone": "Europe/Berlin", "continent": "Europe", "continent_code": "EU",
    "accuracy_radius_km": 50,
    "registered_country": "Germany", "registered_country_iso": "DE"
  },
  "network": {
    "asn": 62336, "org": "PURtel.com GmbH", "prefix": "5.63.60.0/22",
    "provider": null, "service": null, "region": null,
    "classification": {
      "type": "residential",
      "is_datacenter": false, "is_vpn": false, "is_tor": false,
      "is_proxy": false, "is_bot": false, "is_threat": false
    }
  },
  "user_agent": { "raw": "curl/8.7.1", "browser": {}, "os": {}, "device": {} }
}
```

### Sprint 2 — Low-Effort Backend Wins (v0.9.0)

#### 50. ASN routing prefix

`network.prefix` (e.g. `"203.0.113.0/24"`) populated from MaxMind's `LookupResult::network()`.

#### 51. Surface additional GeoIP fields

`location.registered_country` and `location.registered_country_iso` added. Differs from `location.country` for VPN exit nodes — useful as a VPN-detection signal.

#### 19. Individual sub-field endpoints

`/country`, `/city`, `/asn`, `/timezone`, `/latitude`, `/longitude`, `/region` — all support format suffixes and `?ip=`. `/asn` returns `AS64496` in plain text, the raw number in JSON.

#### 48. `ETag` / `Last-Modified` headers

`ETag: "<geoip-build-epoch>"` and `Last-Modified: <http-date>` on all success responses. `304 Not Modified` returned when `If-None-Match` or `If-Modified-Since` matches.
