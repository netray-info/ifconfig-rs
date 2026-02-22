# Roadmap — ifconfig-rs

---

## Future Work

#### 22. Port reachability check

New endpoint `/port/{number}` that attempts a TCP connect back to the requester's IP. Returns open/closed/filtered. Needs a short timeout, a port allowlist/denylist to avoid abuse, and outbound TCP from the host. Only meaningful for the caller's own IP — incompatible with `?ip=`.

---

## Won't Do

### 46. Batch lookup UI

The primary consumer of `POST /batch` is scripts and API clients, not browser users. The overlap with the SPA audience is too small to justify the UI surface area.

### 47. CSV export / download button

The backend already serves `/all/csv` directly. A download button adds negligible value over linking or curling the endpoint.

### 33. WebRTC leak detection

Browsers are progressively restricting WebRTC IP exposure (Firefox blocks by default). Detection reliability will only decline.

### 53. Populate `network.classification.is_proxy`

No free, maintained proxy IP list exists comparable to the VPN/datacenter sources. ASN heuristics would have high false-positive risk. VPN and datacenter flags already cover most of the signal. `is_proxy` has been removed from the API.

### 21. IP decimal representation

IPv4 is trivial; IPv6 exceeds `Number.MAX_SAFE_INTEGER` and would require string serialization, creating an inconsistency. `/ip/cidr` covers the main use case.

### 25. Whois lookup

Most useful WHOIS signal (org, ASN, prefix) is already in `network` from MaxMind. Querying WHOIS servers at request time is slow and rate-limited; a library adds significant dependency weight.

### 39. Code coverage reporting

The test suite (~300 tests across unit, integration, E2E) is already comprehensive. tarpaulin is flaky in CI; Codecov adds a third-party service dependency for marginal gain.

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
