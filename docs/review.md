# Dev Review — Mitigation Plan

Comprehensive review of ifconfig-rs across code quality, security, test coverage, UX/UI, and documentation. No critical findings. **37 total**: 4 high, 18 medium, 15 low. **All 37 items resolved.**

| Severity | Total | Resolved | Open |
|----------|-------|----------|------|
| Critical | 0     | 0        | 0    |
| High     | 4     | 4        | 0    |
| Medium   | 18    | 18       | 0    |
| Low      | 15    | 15       | 0    |

---

## Phase 1: Security Hardening ✅

High-priority items that reduce attack surface. Mostly small, isolated changes. **All 7 items resolved.**

### M2. No Request Body Size Limit — ✅ Resolved

**Severity**: Medium | **Category**: Security

**Problem**: No `DefaultBodyLimit` layer configured. The batch endpoint reads the entire body as `Bytes` (`src/routes.rs:753`). A multi-GB JSON payload would be buffered and deserialized before any size check.

**Resolution**: Added `DefaultBodyLimit::max(1_048_576)` (1 MB) as the innermost layer in `build_app()`. Commit `2e3f277`.

---

### M4. IPv6 Private Address Validation Gap — ✅ Resolved

**Severity**: Medium | **Category**: Security

**Problem**: `is_global_ip()` accepted all non-loopback/unspecified IPv6 addresses including ULA (`fc00::/7`), link-local (`fe80::/10`), and IPv4-mapped private addresses (`::ffff:10.0.0.1`). Combined with `?dns=true`, this enables internal DNS enumeration.

**Resolution**: Extended `is_global_ip()` to reject ULA, link-local, multicast, and IPv4-mapped private addresses. Added 2 new unit tests (10 assertions). Commit `529e451`.

---

### M1. Batch Endpoint Bypasses Rate Limiting — ✅ Resolved

**Severity**: Medium | **Category**: Security

**Problem**: `/batch` is exempt from middleware rate limiting (`src/middleware.rs:52`). The internal `check_key_n` call silently ignored `InsufficientCapacity` errors, processing requests anyway when `max_size > per_ip_burst`.

**Resolution**: `Err(_insufficient)` arm now returns 429 with `X-RateLimit-*` and `Retry-After` headers, consistent with the `Ok(Err(not_until))` arm. Removed dead `rate_ok` variable. Commit `ca5b1e1`.

---

### M3. Missing Content-Security-Policy Header — ✅ Resolved

**Severity**: Medium | **Category**: Security

**Problem**: `security_headers` middleware set X-Content-Type-Options, X-Frame-Options, HSTS, Referrer-Policy, but no CSP. The Scalar docs page loaded JS from `cdn.jsdelivr.net` without SRI hash.

**Resolution**: Added CSP header to all responses. `/docs` path allows `script-src https://cdn.jsdelivr.net`; all other paths restrict to `script-src 'self'` only. Combined with L10 in commit `b8c9203`.

---

### L5. `Location::unknown()` Uses Strings Instead of None — ✅ Resolved

**Severity**: Low | **Category**: Code Quality / API Design

**Problem**: `Location::unknown()` and `Isp::unknown()` set fields to `Some("unknown")` instead of `None`. API consumers couldn't distinguish "no data" from a literal value.

**Resolution**: Changed to `None` for all string fields. JSON output now returns `null` instead of `"unknown"`. Plain text unchanged (handlers use `unwrap_or`). Commit `e205178`.

---

### L9. Batch Input Reflected Unsanitized in Error Responses — ✅ Resolved

**Severity**: Low | **Category**: Security

**Problem**: Batch error responses echoed `ip_str` verbatim. Very long strings amplify response size.

**Resolution**: Truncated reflected input to 45 characters (max length of a valid IPv6 address) in both batch error paths. Commit `ef5997d`.

---

### L10. CDN Script Without SRI Hash — ✅ Resolved

**Severity**: Low | **Category**: Security

**Problem**: `scalar_docs.html` loaded Scalar from jsdelivr without an `integrity` attribute. CDN compromise would execute arbitrary JS.

**Resolution**: Pinned `@scalar/api-reference` to v1.44.25 with `integrity="sha384-..."` and `crossorigin="anonymous"`. Combined with M3 in commit `b8c9203`.

---

## Phase 2: Correctness & Reliability ✅

Fixes for correctness bugs, data loss risks, and reliability issues. **All 5 items resolved.**

### ~~H1. Synchronous Blocking I/O in Async Context~~ — RESOLVED

Converted all 10 backend `from_file()`/`new()`/`from_yaml()` methods from `std::fs` to `tokio::fs`. All callers updated to async. Commit `d17efd7`.

---

### ~~L1. Spamhaus DROP Inline Comments Silently Dropped~~ — RESOLVED

Added `line.split(" ;").next()` to strip inline comments before parsing. Added `parses_inline_comments` test. Commit `5d74de0`.

---

### ~~L2. X-Request-Id Not Propagated to Request Headers~~ — RESOLVED

Made `req` mutable and insert generated ID into request headers before `next.run(req)`, so `TraceLayer` spans include the ID. Commit `da4fabd`.

---

### ~~L7. DNS PTR Lookup Has No Timeout~~ — RESOLVED

Wrapped DNS PTR lookup in `tokio::time::timeout(Duration::from_secs(2))`. Timeout returns `None` (no host). Commit `52a9dc2`.

---

### ~~M11. `u64` to `u32` Truncation in Rate Config~~ — RESOLVED

Changed `per_ip_per_minute` config field from `u64` to `u32`. Removed `as u32` cast in `state.rs`. Commit `e03bde4`.

---

## Phase 3: Code Quality & Cleanup ✅

Internal quality improvements. No user-visible behavior changes. **All 6 items resolved.**

### ~~M5. Manual Config Clone is Fragile~~ — RESOLVED

Derived `Clone` on `Config` and replaced manual field-by-field clone with `config.clone()`. Part of H1 commit `d17efd7`.

---

### ~~M6. Duplicate `RustEmbed` Derivation~~ — RESOLVED

Removed the duplicate `Assets` struct derivation inside `serve_spa()`, keeping the module-level one. Commit `639448f`.

---

### ~~M7. Dead Code (6 items)~~ — RESOLVED

Removed `not_found_handler()`, unused `AppError` variants, and gated test-only `OutputFormat` methods with `#[cfg(test)]`. Commit `5bd6e96`.

---

### ~~M8. `is_proxy` Always False~~ — RESOLVED

Added `TODO` explaining the field is kept for future use but has no data source yet. Commit `7e70be9`.

---

### ~~M10. Route Handler Boilerplate~~ — RESOLVED

Extracted `standard_endpoint!` macro to generate handler pairs, eliminating ~500 lines of duplication. Commit `009cf0c`.

---

### ~~L14. Unused `_state` Parameter~~ — RESOLVED

Removed unused `_state: AppState` parameter from `router()` and updated route registration. Commit `779c38e`.

---

## Phase 4: Dependencies & Build

Dependency hygiene, CI improvements, and supply-chain hardening.

### ~~M9. `serde_yaml` is Unmaintained~~ — RESOLVED

Migrated to `serde_yaml_ng` 0.10 via Cargo rename trick. Note: the originally suggested `serde_yml` has RUSTSEC-2025-0068 (unsoundness) and is archived.

---

### ~~L6. `tokio = { features = ["full"] }`~~ — RESOLVED

Replaced with specific features: `fs`, `macros`, `net`, `rt-multi-thread`, `signal`, `sync`, `time`.

---

### ~~L15. Typo in CI Job Name~~ — RESOLVED

Fixed "Cargo Ftm" → "Cargo Fmt".

---

### ~~M12. CI Does Not Run Integration Tests~~ — RESOLVED

Added integration test step running `ok_handlers`, `error_handler`, and `rate_limit` after unit tests.

---

### ~~L13. CI Action Versions Not SHA-Pinned~~ — RESOLVED

SHA-pinned `actions/checkout` (v4.3.1) and `actions/setup-node` (v4.4.0) across both workflows. Also upgraded checkout from v3 to v4.

---

### ~~L12. Koyeb CLI Installed via Piped Curl~~ — RESOLVED

Pinned to Koyeb CLI v5.9.1 via direct release tarball download instead of `curl | sh`.

---

## Phase 5: Frontend Improvements ✅

Accessibility, SEO, and UX polish for the SolidJS SPA. **All 6 items resolved.**

### ~~H2 + H3. ARIA Labels and Focus Styles~~ — RESOLVED

Added `aria-label` to all icon-only buttons (theme toggle, copy IP, copy hostname, copy curl). Added `aria-expanded`/`aria-controls` to all collapsible sections (Request Headers, API Explorer, FAQ). Added `role="status"` to loading spinner. Added `:focus-visible` outline styles using `--accent`. Commit `0d77e2c`.

---

### ~~H4. Missing SEO Meta Tags~~ — RESOLVED

Added meta description, theme-color (dark/light media queries), Open Graph tags (og:type, og:title, og:description), and Twitter Card tags. Omitted og:url and canonical link since base_url is dynamic per instance. Commit `80c4132`.

---

### ~~M14. Theme Flash of Unstyled Content~~ — RESOLVED

Added inline script in index.html that reads localStorage and sets data-theme synchronously before rendering.

---

### ~~M15. No Error Boundary~~ — RESOLVED

Added SolidJS ErrorBoundary in index.tsx wrapping the app root with a user-friendly fallback message.

---

### ~~M16. Light Mode Contrast Failure~~ — RESOLVED

Darkened light-mode --text-muted from #a1a1aa to #6b6b73 (~5:1 contrast ratio, WCAG AA compliant).

---

### ~~M17. Missing Tablet Breakpoint~~ — RESOLVED

Added @media (max-width: 900px) breakpoint transitioning cards from 3-column to 2-column on tablet viewports.

---

## Phase 6: Test Coverage

Fill gaps identified in the coverage analysis. These can be tackled incrementally.

### ~~6.1 `handlers.rs` Unit Tests~~ — RESOLVED

Added 25 tests covering all to_json/to_plain functions and None paths for tcp, host, network, user_agent.

### ~~6.2 `middleware.rs` Unit Tests~~ — RESOLVED

Added unit tests for generate_request_id (format and uniqueness). Security headers and request ID propagation covered in 6.4 integration tests.

### ~~6.3 `config.rs` Unit Tests~~ — RESOLVED

Added 6 tests covering Config::load defaults, missing file error, sub-config defaults, and TOML round-trip.

### ~~6.4 Security Header Integration Assertions~~ — RESOLVED

Added 5 integration tests asserting CSP, HSTS, X-Frame-Options, X-Content-Type-Options, Referrer-Policy, request ID generation, and request ID propagation.

### ~~6.5 Batch `max_size` Rejection Test~~ — RESOLVED

Added test sending 101 IPs (exceeding default max_size=100), asserting 400 with "exceeds maximum".

### ~~6.6 `/ipv6` Endpoint Tests~~ — RESOLVED

Added 4 integration tests: IPv4 client 404, JSON 404, ?ip= with IPv6 succeeds, ?ip= with IPv4 on /ipv6 returns 404.

### ~~6.7 E2E: Use `baseURL` Instead of Hardcoded Prod URL~~ — RESOLVED

Tests now use page.goto('/') with configurable baseURL (env: BASE_URL). Rewrote tests to match current SolidJS selectors.

---

## Phase 7: Documentation ✅

**All items resolved.**

### ~~M13. CLAUDE.md Inaccuracies~~ — RESOLVED

Fixed 7 inaccuracies: make targets, component list, `/meta` endpoint, `/batch` rate-limit exemption, hosting provider count, test counts, E2E baseURL description. Commit `f6f23ec`.

---

### ~~M18. No CONTRIBUTING.md~~ — RESOLVED

Created `CONTRIBUTING.md` covering data directory setup, build instructions, test commands, and PR workflow. Commit `802abe6`.

---

### ~~L15. CI Typo (duplicate)~~ — RESOLVED

Already tracked and resolved in Phase 4.

---

## Additional Low-Priority Items ✅

**All 4 items resolved.**

### ~~L3. Rate Limiter State Grows Unboundedly~~ — RESOLVED

Spawn background task in `build_app` that calls `retain_recent()` + `shrink_to_fit()` every 5 minutes. Commit `51ce7bd`.

### ~~L4. Sequential Batch Processing~~ — RESOLVED

Replaced sequential for-loop with `tokio::task::JoinSet` for concurrent IP lookups. Results collected back in original order via indexed slots. Commit `0fafdee`.

### ~~L8. Header Filter Uses `Vec<Regex>` Instead of `RegexSet`~~ — RESOLVED

Replaced `Arc<Vec<Regex>>` with `Arc<RegexSet>` in AppState. Single NFA pass instead of iterating individual patterns. Commit `4b605ad`.

### ~~L11. Admin `/metrics` Has No Auth~~ — RESOLVED

Added startup warning when `admin_bind` is configured to a non-loopback address, alerting operators to ensure network-level access control. Commit `3da39ca`.

---

## Suggested Implementation Order

1. ~~**Phase 1** — Security hardening.~~ ✅ Complete (7/7 items resolved).
2. ~~**Phase 2** — Correctness & reliability.~~ ✅ Complete (5/5 items resolved).
3. ~~**Phase 4** — Dependencies & build.~~ ✅ Complete (6/6 items resolved).
4. ~~**Phase 3** — Code quality & cleanup.~~ ✅ Complete (6/6 items resolved).
5. ~~**Phase 5** — Frontend improvements.~~ ✅ Complete (6/6 items resolved).
6. ~~**Phase 6** — Test coverage.~~ ✅ Complete (7/7 items resolved).
7. ~~**Phase 7** — Documentation.~~ ✅ Complete (3/3 items resolved).
