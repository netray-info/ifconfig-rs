# Roadmap — ifconfig-rs

## Done

### 1. CORS headers

~~Add `Access-Control-Allow-Origin: *` on API responses via a new response fairing.~~ Added to the existing `SecurityHeaders` response fairing.

### 2. Expose existing MaxMind data (ASN number, ISO country code, timezone)

~~Thread these through the `Ifconfig` struct and expose on `/location` and `/json`.~~ Added `country_iso`, `timezone`, `continent`, `continent_code` to `Location`; `asn` to `Isp`. Plain text formats updated: `/location` shows `City, Country (ISO), Continent, Timezone`; `/isp` shows `ISP (ASN)`.

### 3. Request headers echo endpoint (`/headers`)

~~Let users see exactly what headers their client is sending.~~ New `/headers` endpoint with full content negotiation (CLI, JSON, plain text, `/headers/json`). Uses a `RequestHeaders` guard. Plain text returns `Header: value` lines; JSON returns a sorted object.

### 6. Copy-to-clipboard on the frontend

~~Add a small clipboard button next to the IP address in the HTML UI. Table-stakes UX for this type of tool. Pure JS, no dependencies needed.~~ Added a copy button inline with the IP address using UIkit's `copy` icon and `navigator.clipboard.writeText()`. Visual feedback turns the button green briefly after copying. Works on both desktop and touch layouts.

### 7. Dark mode

~~Add `prefers-color-scheme: dark` media query support or a toggle.~~ Added automatic dark mode via `@media (prefers-color-scheme: dark)` with a deep navy palette. Covers all UI elements including tabs, code blocks, accordion, and GitHub corner. No toggle — follows OS preference.

### 8. `/health` endpoint

~~A proper health check endpoint that verifies GeoIP databases are loaded and the service is functional.~~ `GET /health` returns `200 {"status": "ok"}` when both GeoIP databases are loaded, or `503 {"status": "unhealthy", "reason": "..."}` when databases are missing. JSON-only, no content negotiation.

### 10. Response caching headers

~~Add `Cache-Control` headers (e.g., `max-age=60` for JSON, `no-cache` for HTML).~~ `SecurityHeaders` fairing now sets `Cache-Control: private, max-age=60` on cacheable API responses (JSON, plain text) and `no-cache` on HTML pages, `/health`, and error responses. All responses include `Vary: Accept, User-Agent`.

### 4. `/all` plain-text endpoint

~~A single CLI-friendly endpoint that dumps everything in a `key: value` format.~~ New `/all` endpoint with full content negotiation. Plain text returns aligned `key: value` pairs (ip, version, hostname, location, ISP, port, browser, OS). Lines are omitted when data is absent (no GeoIP DB). JSON returns the full `Ifconfig` struct.

### 5. IPv4/IPv6 awareness

~~Add `/ipv4` and `/ipv6` sub-endpoints.~~ New `/ipv4` and `/ipv6` endpoints return the client IP only when the connection matches the requested protocol version, otherwise 404. Full content negotiation (CLI, JSON, plain text, `/ipv4/json`, `/ipv6/json`). Uses a new `ip_version_route!` macro.

### 11. Tor exit node detection

~~A boolean `is_tor` field indicating whether the requesting IP is a known Tor exit node.~~ Added `is_tor: Option<bool>` to the `Ifconfig` struct. Loads a local plain-text exit node list (one IP per line) at startup via `TorExitNodes` backed by `HashSet<IpAddr>`. When no list is configured, `is_tor` is `null` in JSON and omitted from `/all` plain text. Appears in all JSON responses and the `/all` plain-text dump. HTML template shows a warning indicator when `is_tor` is true.

### 14. Configurable output formats (YAML, TOML, CSV)

~~Beyond JSON and plain text, some automation tools prefer YAML or other formats.~~ Added YAML, TOML, and CSV output formats to all endpoints. Formats can be requested via `Accept` header (`application/yaml`, `application/toml`, `text/csv`) or URL suffix (`/ip/yaml`, `/ip/toml`, `/ip/csv`). New `src/format.rs` module handles serialization. TOML output strips null fields (TOML has no null type). CSV output flattens nested JSON into dot-notation `key,value` rows. Content negotiation rank order: CLI(1) > JSON(2) > YAML(3) > TOML(4) > CSV(5) > plain(6) > HTML(7). Dependencies: `serde_yaml`, `toml`.

### 9. Rate limiting

~~No rate limiting currently exists. A simple token-bucket or sliding-window limiter would protect against abuse.~~ Added per-IP rate limiting via a `RateLimited` request guard backed by a fixed-window counter using `std::sync` primitives (no external dependencies). Default: 60 requests per 60-second window. Rate limit state is cached per-request to avoid double-counting during Rocket's ranked route forwarding. Expired entries are cleaned up periodically. `/health` is exempt. Returns `429 Too Many Requests` when exceeded.

---

## Won't Do

### 12. JSONP callback support

A `?callback=fn` parameter that wraps JSON responses for legacy environments without CORS. JSONP is a legacy pattern largely obsoleted by CORS (item 1). Adding CORS headers is the modern solution; maintaining a JSONP code path adds complexity for a shrinking audience.

### 13. Prometheus metrics endpoint (`/metrics`)

Request counts by endpoint, response latency histograms, GeoIP lookup durations via `rocket_prometheus` or similar. Adds a non-trivial dependency and operational surface for a feature that only benefits self-hosters running full monitoring stacks. Out of scope for the core service.
