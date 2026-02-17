# Roadmap — ifconfig-rs

## Done

### 1. CORS headers

~~Add `Access-Control-Allow-Origin: *` on API responses via a new response fairing.~~ Added to the existing `SecurityHeaders` response fairing.

### 2. Expose existing MaxMind data (ASN number, ISO country code, timezone)

~~Thread these through the `Ifconfig` struct and expose on `/location` and `/json`.~~ Added `country_iso`, `timezone`, `continent`, `continent_code` to `Location`; `asn` to `Isp`. Plain text formats updated: `/location` shows `City, Country (ISO), Continent, Timezone`; `/isp` shows `ISP (ASN)`.

### 3. Request headers echo endpoint (`/headers`)

~~Let users see exactly what headers their client is sending.~~ New `/headers` endpoint with full content negotiation (CLI, JSON, plain text, `/headers/json`). Uses a `RequestHeaders` guard. Plain text returns `Header: value` lines; JSON returns a sorted object.

---

## Planned

### 4. `/all` plain-text endpoint

A single CLI-friendly endpoint that dumps everything in a `key: value` format:

```
ip:        93.184.216.34
version:   4
hostname:  dns.google
city:      Mountain View
country:   US
timezone:  America/Los_Angeles
asn:       AS15169
isp:       GOOGLE
latitude:  37.386
longitude: -122.0838
browser:   Chrome 120.0.0
os:        Mac OS X 14.2.1
```

Popular on similar services for quick diagnostic scripts.

### 5. IPv4/IPv6 awareness

Add `/ipv4` and `/ipv6` sub-endpoints or surface the protocol version more prominently. Network engineers in dual-stack environments need to verify which protocol path their traffic takes. The IP version field exists but isn't shown in the plain-text root response.

### 6. Copy-to-clipboard on the frontend

Add a small clipboard button next to the IP address in the HTML UI. Table-stakes UX for this type of tool. Pure JS, no dependencies needed.

### 7. Dark mode

Add `prefers-color-scheme: dark` media query support or a toggle. The UIkit-based frontend is light-only. Most developers (the primary audience) use dark mode.

### 8. `/health` endpoint

A proper health check endpoint that verifies GeoIP databases are loaded and the service is functional. Useful for Docker health checks, Kubernetes probes, and monitoring for self-hosters.

### 9. Rate limiting

No rate limiting currently exists. A simple token-bucket or sliding-window limiter would protect against abuse. Prefer a lightweight implementation using `std::sync` primitives over adding a heavy dependency.

### 10. Response caching headers

Add `Cache-Control` headers (e.g., `max-age=60` for JSON, `no-cache` for HTML). GeoIP data doesn't change per-second. Reduces redundant lookups and improves performance for API consumers.

### 11. Tor/VPN/proxy detection

A boolean `is_tor` or `is_proxy` field. Tor exit node lists are publicly available and could be loaded similarly to the GeoIP databases. Useful for security-conscious API consumers.

### 14. Configurable output format (`?format=yaml|toml|csv`)

Beyond JSON and plain text, some automation tools prefer YAML or other formats. Low priority but occasionally requested on similar services.

---

## Won't Do

### 12. JSONP callback support

A `?callback=fn` parameter that wraps JSON responses for legacy environments without CORS. JSONP is a legacy pattern largely obsoleted by CORS (item 1). Adding CORS headers is the modern solution; maintaining a JSONP code path adds complexity for a shrinking audience.

### 13. Prometheus metrics endpoint (`/metrics`)

Request counts by endpoint, response latency histograms, GeoIP lookup durations via `rocket_prometheus` or similar. Adds a non-trivial dependency and operational surface for a feature that only benefits self-hosters running full monitoring stacks. Out of scope for the core service.
