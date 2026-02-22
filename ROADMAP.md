# Roadmap — ifconfig-rs

## Sprint 1 — API v2: Data Model Cleanup

> Breaking changes. Must ship as a single version bump with migration notes. All three items touch the same serialization boundary — doing them separately would require multiple breaking releases.

#### 40. Merge `isp` into `network`

`isp.name` and `isp.asn` are AS-level network attributes. `network.provider` (used for cloud/VPN/bot) and `isp.name` are the same concept for different network types — unify them. Rename `isp.name` → `network.org`, `isp.asn` → `network.asn`. Remove the `Isp` struct and top-level `isp` key. Fold `/isp` into `/network` or keep as alias. Update `InfoCards`: move ASN and org rows into the Network card.

#### 41. Move `host` into `ip`

The PTR/reverse-DNS hostname is a property of the IP address. Flatten `host.name` → `ip.hostname` (nullable string). Remove the `Host` struct and top-level `host` key. Update `InfoCards` accordingly.

#### 42. Move `user_agent_header` into `user_agent.raw`

`user_agent_header` is the raw source for the parsed `user_agent` — it should live as `user_agent.raw`. When no UA is present the entire `user_agent` object (including `raw`) remains null. No UI change needed.

Target shape:

```json
{
  "ip":       { "addr": "5.63.60.1", "version": "4", "hostname": "5-63-60-1.example.net" },
  "tcp":      { "port": 4810 },
  "location": { "..." },
  "network":  { "type": "residential", "asn": 62336, "org": "PURtel.com GmbH",
                "provider": null, "service": null, "region": null,
                "is_datacenter": false, "is_vpn": false, "is_tor": false,
                "is_proxy": false, "is_bot": false, "is_threat": false },
  "user_agent": { "raw": "curl/8.7.1", "browser": {}, "os": {}, "device": {} }
}
```

---

## Sprint 2 — Low-Effort Backend Wins

> All items use data already loaded or computed — no new dependencies or data sources required.

#### 50. ASN routing prefix

The BGP prefix is already computed during MaxMind lookup but discarded. Add `prefix: Option<String>` to `Isp` (e.g. `"203.0.113.0/24"`) — `ipnetwork` is already a transitive dep. See Implementation Notes.

#### 51. Surface additional GeoIP fields

`registered_country` (free, strong VPN-detection signal — differs from `location.country` for VPN exit nodes) and `geoname_id` on country/city (enables external links to GeoNames/OpenStreetMap). See Implementation Notes for full field inventory.

#### 19. Individual sub-field endpoints

Expose each field as a standalone plain-text endpoint: `/country`, `/city`, `/asn`, `/timezone`, `/latitude`, `/longitude`, `/region`. Data already computed — thin route+handler wrappers. Enables `curl ip.pdt.sh/country` → `Germany`.

#### 48. `ETag` / `Last-Modified` headers

Add `ETag` and `Last-Modified` response headers to enable `304 Not Modified`. The GeoIP database build epoch is a natural `Last-Modified` value.

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

#### 37. Add E2E tests to CI

Playwright tests exist but are not in the GitHub Actions pipeline. Add a CI job that runs them against the Docker image.

#### 38. Dependency vulnerability scanning

Add `cargo-audit` (Rust) and `npm audit` (frontend) to the CI pipeline.

#### 49. Enrichment quality Prometheus gauges

Track null-rate per field as a gauge. Useful for detecting GeoIP database staleness or missing data sources in production without inspecting individual responses.

#### 52. Data file acquisition docs

Document where to obtain required data files: GeoLite2-City/ASN (MaxMind account + `geoipupdate`), Tor exit nodes, cloud/VPN/bot/datacenter ranges, Feodo, Spamhaus DROP. Include example `geoipupdate.conf` and a `make update-data` target.

---

## Sprint 5 — Advanced Backend

#### 43. Internal mode

A config flag (`internal_mode = true`) that allows the service to respond to requests from private and reserved IP ranges. Affected subnets: RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), loopback (`127.0.0.0/8`, `::1/128`), link-local (`169.254.0.0/16`, `fe80::/10`), ULA (`fc00::/7`). GeoIP lookups return no results (expected); `network.type` reflects `"internal"`. The `extractors.rs` global-IP guard and `get_ifconfig()` validation must be conditioned on this flag via `AppState`.

#### 22. Port reachability check

New endpoint `/port/{number}` that attempts a TCP connect back to the requester's IP. Returns open/closed/filtered. Needs a timeout and connection limits to avoid abuse. This has to be discussed first, before it gets implemented, because I dont know if I want it.

#### 23. DNS reverse lookup caching

Reverse DNS lookup has no caching. Add an in-memory LRU cache with a short TTL (e.g., 60s).

#### 44. Nested `?fields=` dot-notation

Extend `?fields=` to support dot-notation for sub-fields (e.g., `?fields=location.city,network.asn`). Currently `filter_fields()` in `format.rs` only handles top-level JSON keys. This has to be discussed first, before it gets implemented, because I dont know if I want it.

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

### ASN Routing Prefix in ISP Data

**The prefix is already computed — it's just discarded.** `maxminddb`'s `LookupResult::network()` returns the BGP routing prefix as `ipnetwork::IpNetwork` (a transitive dep). No new direct dependency needed.

**Changes required:**

| File | Change |
|---|---|
| `src/backend/mod.rs:63–72` | Change `lookup` to return `Option<(geoip2::Isp<'_>, Option<String>)>` — keep `LookupResult`, call `.network().map(\|n\| n.to_string())` |
| `src/backend/mod.rs:155–170` | Add `prefix: Option<String>` to `Isp` struct; update `Isp::unknown()` |
| `src/backend/mod.rs:289–296` | Populate `isp.prefix` in `get_ifconfig()` |
| `src/handlers.rs:104–111` | `isp::to_plain` — new format: `"Example Telecom (AS64496, 203.0.113.0/24)\n"` |
| `src/handlers.rs:223–228` | `all::to_plain` — add `"prefix:     {}\n"` line after ASN |
| `frontend/src/lib/types.ts` | Add `prefix: string \| null` to `Isp` interface |
| `frontend/src/components/InfoCards.tsx` | Add `<Show when={isp().prefix != null}>` row |

---

### Unused GeoIP Fields

**GeoLite2-City (free, high value):**

| Field | Practical use |
|---|---|
| `registered_country` | Country where IP block was registered — differs from location for VPNs; strong proxy/VPN signal |
| `geoname_id` on country/city | External links to GeoNames, Wikipedia, OpenStreetMap |
| `represented_country` + `type` | Embassy/military detection (`"military"`, `"diplomatic"`) |
| `subdivisions[1..]` | County/district below state |
| Non-English `names` | Localised names in DE, ES, FR, JA, PT-BR, RU, ZH-CN |

**GeoLite2-ISP (paid tier only):** `isp` brand name, `organization`, `mobile_country_code`, `mobile_network_code`.

**Skip:** `metro_code` (deprecated), `AnonymousIp` DB (redundant with existing CIDR files), confidence scores (Enterprise-only).

---

## Won't Do

### 12. JSONP callback support

JSONP is obsoleted by CORS. Maintaining a JSONP code path adds complexity for a shrinking audience.

### 13. Prometheus metrics on the main port

The admin port (`server.admin_bind`) already exposes `/metrics` for those who need it.
