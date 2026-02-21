# Dev Review — Mitigation Plan

Comprehensive review of ifconfig-rs across code quality, security, test coverage, UX/UI, and documentation. No critical findings. **37 total**: 4 high, 18 medium, 15 low. **7 resolved** (Phase 1 complete).

| Severity | Total | Resolved | Open |
|----------|-------|----------|------|
| Critical | 0     | 0        | 0    |
| High     | 4     | 0        | 4    |
| Medium   | 18    | 4        | 14   |
| Low      | 15    | 3        | 12   |

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

## Phase 2: Correctness & Reliability

Fixes for correctness bugs, data loss risks, and reliability issues.

### H1. Synchronous Blocking I/O in Async Context — Effort: M

**Severity**: High | **Category**: Code Quality / Performance

**Problem**: All backend `from_file()` methods use `std::fs::read_to_string` but are called from the async `EnrichmentContext::load()`. This blocks a tokio worker thread during startup and SIGHUP reload.

**Mitigation**: Use `tokio::fs::read_to_string` or wrap the file reads in `tokio::task::spawn_blocking`.

**Files**: `src/enrichment.rs`, `src/backend/cloud_provider.rs:26`, `src/backend/vpn.rs:10`, `src/backend/bot.rs:22`, `src/backend/datacenter.rs:10`, `src/backend/spamhaus.rs:10`, `src/backend/feodo.rs:8`, `src/backend/mod.rs:76`

---

### L1. Spamhaus DROP Inline Comments Silently Dropped — Effort: S

**Severity**: Low | **Category**: Code Quality

**Problem**: `src/backend/spamhaus.rs:14-19` doesn't handle ` ; SB123456` inline comments. Lines with trailing comments fail `parse::<IpNetwork>()` and are silently dropped, reducing coverage.

**Mitigation**: Strip inline comments (everything after ` ;`) before parsing.

**Files**: `src/backend/spamhaus.rs`

---

### L2. X-Request-Id Not Propagated to Request Headers — Effort: S

**Severity**: Low | **Category**: Observability

**Problem**: Generated request IDs are only set on the response (`src/middleware.rs:35-48`). The `TraceLayer` span tries to read from request headers, so server-generated IDs always show as `"-"` in logs.

**Mitigation**: Also insert generated request IDs into the request headers before the span is created.

**Files**: `src/middleware.rs`

---

### L7. DNS PTR Lookup Has No Timeout — Effort: S

**Severity**: Low | **Category**: Reliability / Performance

**Problem**: `src/backend/mod.rs:230-245` has no explicit timeout on DNS lookups. A slow DNS server stalls request handling.

**Mitigation**: Wrap DNS lookups in `tokio::time::timeout` (e.g., 2 seconds).

**Files**: `src/backend/mod.rs`

---

### M11. `u64` to `u32` Truncation in Rate Config — Effort: S

**Severity**: Medium | **Category**: Code Quality

**Problem**: `src/state.rs:49` casts `per_ip_per_minute` from `u64` to `u32` with silent truncation.

**Mitigation**: Use `u32::try_from().unwrap_or(u32::MAX)` or change the config field type to `u32`.

**Files**: `src/state.rs` or `src/config.rs`

---

## Phase 3: Code Quality & Cleanup

Internal quality improvements. No user-visible behavior changes.

### M5. Manual Config Clone is Fragile — Effort: S

**Severity**: Medium | **Category**: Code Quality

**Problem**: `src/state.rs:96-127` manually clones every `Config` field. Adding a new field requires updating this block or it's silently dropped.

**Mitigation**: Derive `Clone` on `Config` and replace the manual field-by-field clone with `config.clone()`.

**Files**: `src/config.rs`, `src/state.rs`

---

### M6. Duplicate `RustEmbed` Derivation — Effort: S

**Severity**: Medium | **Category**: Code Quality

**Problem**: `Assets` struct is derived twice: `src/routes.rs:130` (inside `serve_spa()`) and `src/routes.rs:1098` (module level). Both compile to the same embed; one is redundant.

**Mitigation**: Remove the duplicate derivation, keep the module-level one.

**Files**: `src/routes.rs`

---

### M7. Dead Code (6 items) — Effort: S

**Severity**: Medium | **Category**: Code Quality

**Problem**: Several functions and enum variants are never used:
- `not_found_handler()` in `src/middleware.rs:144`
- `AppError::NotFound`, `AppError::IpVersionMismatch`, `AppError::Internal` in `src/error.rs:24-29`
- `OutputFormat::from_name`, `OutputFormat::mime_type` in `src/format.rs:13,23` (only used in tests)

**Mitigation**: Remove dead code. Gate test-only items with `#[cfg(test)]`.

**Files**: `src/middleware.rs`, `src/error.rs`, `src/format.rs`

---

### M8. `is_proxy` Always False — Effort: S

**Severity**: Medium | **Category**: Code Quality / API Design

**Problem**: `Network.is_proxy` in `src/backend/mod.rs:382` is hardcoded to `false` everywhere. Dead field that misleads API consumers.

**Mitigation**: Remove the field, or add a `TODO` with the reason it's kept for future use.

**Files**: `src/backend/mod.rs`

---

### M10. Route Handler Boilerplate — Effort: M

**Severity**: Medium | **Category**: Code Quality

**Problem**: ~15 handler pairs (`X_handler` + `X_format_handler`) repeat the same 4-line pattern. ~500 lines of duplication.

**Mitigation**: Extract a macro or factory function to generate handler pairs.

**Files**: `src/routes.rs`

---

### L14. Unused `_state` Parameter — Effort: S

**Severity**: Low | **Category**: Code Quality

**Problem**: `src/routes.rs:56` takes `_state: AppState` that is never used, forcing an unnecessary `.clone()` on every request through the dispatcher.

**Mitigation**: Remove the unused parameter and update the route registration in `src/lib.rs`.

**Files**: `src/routes.rs`, `src/lib.rs`

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

## Phase 5: Frontend Improvements

Accessibility, SEO, and UX polish for the SolidJS SPA.

### H2 + H3. ARIA Labels and Focus Styles — Effort: M

**Severity**: High | **Category**: Accessibility

**Problem (H2)**: Zero `aria-*` attributes in the entire frontend. Icon-only buttons (copy, theme toggle) have no accessible names. Collapsible sections lack `aria-expanded`/`aria-controls`.

**Problem (H3)**: `global.css` has zero `:focus` or `:focus-visible` rules. Keyboard navigation has no visible indicator.

**Mitigation**:
- Add `aria-label` to all icon-only buttons (copy, theme toggle)
- Add `aria-expanded` and `aria-controls` to collapsible/disclosure sections
- Add `role="status"` to loading spinner
- Add `:focus-visible` styles using the existing `--accent` custom property

**Files**: `frontend/src/components/ThemeToggle.tsx`, `frontend/src/components/IpDisplay.tsx`, `frontend/src/components/RequestHeaders.tsx`, `frontend/src/components/ApiExplorer.tsx`, `frontend/src/components/Faq.tsx`, `frontend/src/styles/global.css`

---

### H4. Missing SEO Meta Tags — Effort: S

**Severity**: High | **Category**: SEO

**Problem**: `index.html` is missing `<meta name="description">`, Open Graph tags, Twitter Card tags, `<meta name="theme-color">`, and `<link rel="canonical">`. Hurts discoverability and social sharing for a public service.

**Mitigation**: Add standard SEO meta tags, Open Graph tags (`og:title`, `og:description`, `og:url`, `og:type`), Twitter Card tags, `theme-color`, and canonical link.

**Files**: `frontend/index.html`

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

## Phase 7: Documentation

### M13. CLAUDE.md Inaccuracies — Effort: S

**Severity**: Medium | **Category**: Documentation

Corrections needed:
- `make frontend` → `make frontend-build`
- `make tests` → `make test`
- Frontend component list outdated (says `ApiDocs`, should be `ApiExplorer`; missing `Faq`, `RequestHeaders`)
- `/meta` endpoint undocumented
- `/batch` rate-limit exemption undocumented
- Hosting provider count: ~34 not ~40

**Files**: `CLAUDE.md`

---

### M18. No CONTRIBUTING.md — Effort: S

**Severity**: Medium | **Category**: Documentation

No contributing guide explaining data directory setup, test environment, or PR process.

**Mitigation**: Create `CONTRIBUTING.md` covering data setup, test instructions, and PR workflow.

**Files**: `CONTRIBUTING.md` (new)

---

### L15. CI Typo (duplicate) — Effort: S

Already tracked in Phase 4 (`.github/workflows/ci.yml:46` — "Cargo Ftm" → "Cargo Fmt").

---

## Additional Low-Priority Items

These items are worth tracking but have minimal risk or impact.

| ID  | Finding | Effort |
|-----|---------|--------|
| L3  | Rate limiter state grows unboundedly (`DashMap` has no cleanup) | M |
| L4  | Sequential batch processing (slow with `?dns=true` + 100 IPs) | M |
| L8  | Header filter uses `Vec<Regex>` instead of `RegexSet` | S |
| L11 | Admin `/metrics` has no auth (mitigated by `127.0.0.1` bind) | S |

---

## Suggested Implementation Order

1. ~~**Phase 1** — Security hardening.~~ ✅ Complete (7/7 items resolved).
2. **Phase 2** — Correctness. H1 (spawn_blocking) is the largest item; the rest are small.
3. **Phase 4** — Dependencies. M9 (serde_yaml replacement) is the most involved; CI fixes are quick wins.
4. **Phase 3** — Code cleanup. All items are safe refactors. M10 (handler macro) is the most involved.
5. **Phase 5** — Frontend. H2+H3 (accessibility) has the highest user impact. SEO tags (H4) are a quick win.
6. **Phase 6** — Tests. Tackle alongside the phases they relate to (e.g., write security header integration tests for the new CSP header).
7. **Phase 7** — Documentation. Can be done at any time.
