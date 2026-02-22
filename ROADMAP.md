# Roadmap — ifconfig-rs

Items yet to be decided or implemented.

---

## Bugs

#### B5. Network classification not visible in network card

The `network.classification` sub-object fields (`type`, `is_vpn`, `is_tor`, `is_datacenter`, `is_bot`, `is_threat`, `is_proxy`) are not displayed in the Network card in the SPA.

---

## Pending

#### 22. Port reachability check

New endpoint `/port/{number}` that attempts a TCP connect back to the requester's IP. Returns open/closed/filtered. Needs a timeout and connection limits to avoid abuse.

#### 46. Batch lookup UI

Frontend interface for `POST /batch`. Paste a list of IPs, view results in a table. The batch endpoint already exists.

#### 47. CSV export / download button

The backend already serves `/all/csv`. Add a download button in the SPA to export the current result.

#### 33. WebRTC leak detection

Frontend-only: detect IPs leaked via WebRTC when using a VPN. Strong differentiator for privacy-conscious users.

#### 53. Populate `network.classification.is_proxy`

`is_proxy` is currently always `false` — no proxy detection data source is wired up. Options: integrate a proxy IP list (similar to `vpn_ranges.txt`), use a third-party API, or derive from existing heuristics (e.g. known open-proxy ASNs). Needs a data source decision before implementation.

#### 21. IP decimal representation

Add `ip_decimal` to the `Ip` struct — the integer representation (e.g., `1.2.3.4` = `16909060`).

#### 25. Whois lookup

Return WHOIS registration data for the IP's network block. Requires calling WHOIS servers or integrating a library. `mhost` has this functionality but it's a heavy dependency.

#### 39. Code coverage reporting

Add tarpaulin (Rust) and c8 (JS) with reporting to Codecov or similar.

---

## Won't Do

### 29. Map visualization

Lat/long are returned and already linked to Google Maps. Embedding a map (Leaflet + OSM) adds a non-trivial dependency for marginal value.

### 24. Multi-language location names

Implemented `?lang=` query parameter with `pick_name()` helper and a language selector dropdown, then reverted — the UX added noise without clear value.

### 44. Nested `?fields=` dot-notation

Complexity cost (recursive descent in `filter_fields()`, special-casing in plain-text and CSV serialisers) outweighs the benefit; `jq` covers the use case on the client side.

### 12. JSONP callback support

JSONP is obsoleted by CORS.

### 13. Prometheus metrics on the main port

The admin port (`server.admin_bind`) already exposes `/metrics`.

---

## Implementation Notes

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
