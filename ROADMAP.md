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

### 16. Re-wire rate limiting for Axum

~~Rate limiting was implemented for the Rocket-era codebase (item 9) but was lost in the Axum migration.~~ Re-wired per-IP rate limiting using `governor`'s `DefaultKeyedRateLimiter<IpAddr>` with a custom Axum middleware function. Removed unused `tower_governor` dependency. Configurable via `[rate_limit]` in config (default: 60 req/min, burst 10). `/health` is exempt. Returns `429 Too Many Requests` when exceeded. Integration tests cover burst exhaustion and health exemption.

---

## Future

### Bug Fixes / Migration Debt

#### 15. Fix trusted proxies CIDR parsing

Config accepts CIDR ranges like `trusted_proxies = ["10.0.0.0/8"]`, but extractors.rs only does `IpAddr::from_str()` — actual CIDR matching isn't implemented. Needs an `ipnetwork`-style check so proxy networks work correctly.

### Backend — High Value, Low Effort

#### 17. Arbitrary IP lookup

Look up geolocation/ISP/hostname for any IP address, not just the requester's own. All backend machinery (`GeoIpCityDb`, `GeoIpAsnDb`, `TorExitNodes`, `dns_lookup`) already exists — just needs a route like `/lookup/{ip}` or a `?ip=` query parameter. This is the #1 most common feature across competing services (ifconfig.co, ipinfo.io, echoip).

#### 18. Region/state and postal code from MaxMind

The GeoLite2-City database already contains region/state (`subdivisions`) and postal code data, but `get_ifconfig()` doesn't extract them. Add `region`, `region_code`, and `postal_code` fields to the `Location` struct.

#### 19. Individual sub-field endpoints

Expose each location/ISP field as a standalone plain-text endpoint: `/country`, `/city`, `/asn`, `/timezone`, `/latitude`, `/longitude`, `/region`. Data is already computed — just needs thin route+handler wrappers following the existing macro pattern. Enables scripting like `curl ip.pdt.sh/country` → `Germany`.

#### 20. EU membership flag

Add a boolean `eu` field to `Location` based on a static lookup table against `country_iso`. Useful for GDPR-related automation. ifconfig.co has this.

#### 21. IP decimal representation

Add an `ip_decimal` field with the integer representation of the IP address (e.g., `1.2.3.4` = `16909060`). Trivial `u32`/`u128` conversion. Occasionally useful for database storage or comparison.

### Backend — Medium Effort

#### 22. Port reachability check

New endpoint `/port/{number}` that attempts a TCP connect back to the requester's IP on the given port. Returns open/closed/filtered status. Useful for diagnosing firewalls, NAT, port forwarding. Both ifconfig.co and echoip offer this. Needs a timeout and connection limits to avoid abuse.

#### 23. DNS reverse lookup caching

Reverse DNS lookup happens on every request with no caching. Add an in-memory LRU cache with a short TTL (e.g., 60s) to reduce latency and external dependency. DNS can be slow/flaky and may be a bottleneck under load.

#### 24. Multi-language location names

MaxMind's database already contains localized city/country names in ~8 languages. A `?lang=de` query parameter would serve international users without any new data sources.

#### 25. Whois lookup

Return WHOIS registration data for the IP address or its network block. Popular with power users. Requires calling out to WHOIS servers or integrating a library.

#### 26. Privacy/proxy detection beyond Tor

Detect VPN, proxy, relay, or datacenter/hosting IPs. Could start with a free hosting-provider IP list and expand later. ipinfo.io's most popular paid feature. Requires additional data sources.

### Backend — Observability

#### 27. Request logging middleware

`tracing` is already a dependency but there's no HTTP access logging. Add `tower-http`'s `TraceLayer` for structured request logs (method, path, status, duration).

#### 28. Health endpoint detail

`/health` currently only checks GeoIP databases. Report status of all backends (user-agent parser, Tor node list) for a complete picture.

### Frontend

#### 29. Map visualization

Lat/long are already returned by the API. Add a lightweight interactive map (e.g., Leaflet with OpenStreetMap tiles) to the SPA showing the detected location. Strong visual differentiator.

#### 30. Skeleton loading screens

Replace the "Loading..." text with animated placeholder shapes matching the card layout. Better perceived performance.

#### 31. Accessibility (a11y) pass

Add `aria-label` attributes to buttons (copy, theme toggle), `aria-expanded` to collapsible sections (Request Headers, API Explorer, FAQ), `role="tab"`/`role="tabpanel"` to selectors, keyboard navigation management, and a skip-to-content link.

#### 32. Tablet breakpoint

Currently jumps from 3-column to 1-column at 600px. Add a 2-column layout at ~900px for a smoother responsive transition.

#### 33. WebRTC leak detection

Frontend-only feature: detect IP addresses leaked via WebRTC when using a VPN. Strong differentiator for privacy-conscious users. jason5ng32/MyIP has this.

#### 34. Copy UX improvements

Toast notifications instead of just icon color changes on copy. Make the full curl command in the API Explorer selectable and copyable.

### Testing & Infrastructure

#### 35. Frontend component tests

Zero tests exist for SolidJS components. Add Vitest + solid-testing-library for unit/component tests covering IpDisplay, InfoCards, ApiExplorer, ThemeToggle.

#### 36. Expand Playwright E2E suite

Only 1 E2E test exists. Add coverage for theme toggle, FAQ section, error states, mobile responsive layouts, and individual API endpoints.

#### 37. Add E2E tests to CI

Playwright tests exist but aren't in the GitHub Actions pipeline. Add a CI job that runs them.

#### 38. Dependency vulnerability scanning

Add `cargo-audit` (Rust) and `npm audit` (frontend) to the CI pipeline for automated security scanning.

#### 39. Code coverage reporting

No coverage metrics for either Rust or TypeScript. Add tarpaulin (Rust) and Istanbul/c8 (JS) with reporting to Codecov or similar.

---

## Won't Do

### 12. JSONP callback support

A `?callback=fn` parameter that wraps JSON responses for legacy environments without CORS. JSONP is a legacy pattern largely obsoleted by CORS (item 1). Adding CORS headers is the modern solution; maintaining a JSONP code path adds complexity for a shrinking audience.

### 13. Prometheus metrics endpoint (`/metrics`)

Request counts by endpoint, response latency histograms, GeoIP lookup durations via `rocket_prometheus` or similar. Adds a non-trivial dependency and operational surface for a feature that only benefits self-hosters running full monitoring stacks. Out of scope for the core service.
