# SDD: Rename ifconfig-rs to iris

**Status:** Ready for Implementation
**Original:** specs/sdd/rename-ifconfig-rs-to-iris.md
**Refined:** 2026-04-11

## Overview

Rename the ifconfig-rs project to **iris** across the entire netray.info ecosystem. The rename aligns the IP enrichment service with the suite's optics/light naming theme (prism, sight, spectra, beacon, lens) and removes the generic "ifconfig" name inherited from the Unix utility. No API wire format, endpoint paths, or DNS records change — this is a pure renaming across code, config, Docker, CI/CD, deployment, and documentation.

## Context & Constraints

- **Stack**: Rust (Axum 0.8, tokio), SolidJS 1.9 frontend, Vite 6, Docker multi-stage builds
- **DNS stays**: `ip.netray.info` is a service domain, not a project name — it does not change
- **GitHub redirect**: Renaming `lukaspustina/ifconfig-rs` → `lukaspustina/iris` auto-creates a redirect; existing clones continue to push/pull
- **GHCR images**: Old image tags remain accessible; new pushes go to `ghcr.io/netray-info/iris`
- **Coordinated deploy**: Changing the Docker service name requires all sibling services referencing `http://ifconfig-rs:8000` to be redeployed in the same Ansible run
- **No backwards-compat shims**: Clean cut, no transition period (per project conventions)
- **Conventional commits**: Use `refactor:` prefix for rename commits within each repo
- **Scoped changes**: Rename-only commits — no functional changes mixed in
- **Wire format frozen**: JSON field names (`ip`, `location`, `network`, etc.) are unchanged; the `IpInfo` rename is internal only

## Architecture

No architectural changes. This is a pure naming change across:

```
ifconfig-rs repo (iris)
  ├── Rust crate + binary
  ├── SolidJS frontend
  ├── Docker image + compose
  ├── CI/CD workflows
  └── Documentation

Meta repo (netray.info)
  ├── CLAUDE.md suite architecture
  ├── deploy/docker-compose.yml
  ├── site/index.html
  └── specs/

Sibling repos (5 services + 2 libraries)
  ├── Config files (*_url, [backends.ip])
  ├── Docker compose templates
  ├── Documentation
  └── Source code comments

argus-oci (production deployment)
  ├── Ansible role (ifconfig_rs → iris)
  ├── Group vars (ifconfig_rs_* → iris_*)
  ├── Docker compose template
  ├── Config template (ifconfig.toml.j2 → iris.toml.j2)
  ├── Cross-service config templates
  └── Monitoring (OTel collector)
```

## Requirements

1. The Rust crate shall be named `iris` with binary name `iris`.
2. The central data struct shall be renamed from `Ifconfig` to `IpInfo`.
3. The environment variable prefix shall change from `IFCONFIG_` to `IRIS_`.
4. Config files shall be renamed to `iris.dev.toml` and `iris.example.toml`.
5. The Docker image shall be `ghcr.io/netray-info/iris` and the data image `ghcr.io/netray-info/iris-data`.
6. The Ansible role shall be renamed from `ifconfig_rs` to `iris` with all variable prefixes updated from `ifconfig_rs_*` to `iris_*`.
7. All sibling services shall reference Docker hostname `iris` (not `ifconfig-rs`) for internal enrichment calls.
8. The frontend SPA shall display `iris` as the logo text and use `iris_theme` as the `localStorage` key.
9. The OpenAPI spec title shall be `iris`.
10. The `http_metrics` call in `src/middleware.rs` shall pass `"iris"` instead of `"ifconfig"`.
11. The default log filter shall use `iris=debug` instead of `ifconfig_rs=debug`.
12. The deploy webhook path shall change from `deploy-ifconfig-rs` to `deploy-iris`.
13. The GitHub repository shall be renamed from `lukaspustina/ifconfig-rs` to `lukaspustina/iris`.
14. All documentation (README, CLAUDE.md, docs/enrichment.md, data/README.md) shall reference `iris` consistently.
15. Cross-repo documentation and config comments referencing `ifconfig-rs` shall be updated in all sibling repos and the meta repo.
16. mhost-prism's `ifconfig_url` config key shall be renamed to `ip_url` (aligning with all other repos).
17. Grafana dashboards and alert rules in argus-oci shall be updated for the `iris` metrics namespace in the same Ansible deploy as Phase 8.
18. beacon shall be updated to reference `http://iris:8000` for IP enrichment.

## File & Module Structure

No new files or modules. Affected paths listed per phase below.

## Data Models

### Rust

```rust
// src/backend/mod.rs  (was: Ifconfig)
pub struct IpInfo { ... }           // all fields unchanged
pub struct IpInfoParam { ... }      // was: IfconfigParam

// function renames
pub async fn get_ip_info(...) -> Result<IpInfo, ...>   // was: get_ifconfig()
```

### TypeScript

```typescript
// frontend/src/lib/types.ts
export interface IpInfo { ... }     // was: Ifconfig — all fields unchanged

// frontend/src/lib/api.ts
export async function fetchIpInfo(): Promise<IpInfo>          // was: fetchIfconfig
export async function fetchIpInfoForIp(ip: string): Promise<IpInfo>  // was: fetchIfconfigForIp
```

### Config struct

No field changes. `Config::load(Some("iris.dev.toml"))` replaces `Config::load(Some("ifconfig.dev.toml"))` everywhere.

## API Contracts

No changes. All endpoints, HTTP methods, response schemas, and JSON field names are identical. The OpenAPI spec `title` field changes from `"ifconfig-rs"` to `"iris"`. The `components.schemas` key changes from `"Ifconfig"` to `"IpInfo"` as a side-effect of the struct rename (utoipa derives schema names from struct names).

## Configuration

| Key | Old value | New value |
|-----|-----------|-----------|
| Env prefix | `IFCONFIG_` | `IRIS_` |
| Config file | `ifconfig.dev.toml`, `ifconfig.example.toml` | `iris.dev.toml`, `iris.example.toml` |
| `project_name` | `"ifconfig-rs"` | `"iris"` |
| Config comments | Reference `IFCONFIG_SERVER__BIND` | Reference `IRIS_SERVER__BIND` |
| `[telemetry] service_name` | `"ifconfig"` | `"iris"` |

No new config keys. No removed config keys.

## Error Handling

| Failure | Trigger | Behaviour | User-visible |
|---------|---------|-----------|--------------|
| Old `ifconfig.dev.toml` missing after rename | `cargo test` without renaming config | `Config::load` returns `Err`; test panics with "test config" | Test failure message names missing file |
| Old `ifconfig-rs` Docker hostname unreachable | Sibling deployed before iris rename | Connection refused to enrichment endpoint | Sibling returns degraded enrichment (existing error path unchanged) |
| Grafana dashboard uses old `ifconfig` metric namespace | argus-oci deployed without dashboard update | Dashboard panels show "No data" | Ops team must update dashboards in same deploy |
| `iris` container fails health check post-deploy | Bad image or config | Ansible reports task failure | Run rollback: revert Docker image tag in group vars and re-run playbook; expected restart window is under 30 seconds |

## Implementation Phases

Each phase produces a single committable and testable unit. Complete phases in order; sibling repos (Phase 7) and argus-oci (Phase 8) can only be done after Phase 4 (image name finalized).

---

### Phase 1: Rust crate core rename

**Commit message:** `refactor(iris): rename crate, binary, and core types from ifconfig-rs`

**Files to modify:**

| File | Change |
|------|--------|
| `Cargo.toml` | `name = "iris"`, `[[bin]] name = "iris"`, update keywords, description |
| `src/backend/mod.rs` | `Ifconfig` → `IpInfo`; `IfconfigParam` → `IpInfoParam`; `get_ifconfig()` → `get_ip_info()` |
| `src/handlers.rs` | `make_ifconfig()` → `make_ip_info()`; `make_ifconfig_lang()` → `make_ip_info_lang()`; all `&Ifconfig` params → `&IpInfo` |
| `src/routes.rs` | OpenAPI title `"iris"`; all `Ifconfig` type refs → `IpInfo`; `ifconfig` variable names → `ip_info` |
| `src/middleware.rs` | Line 16: `http_metrics("iris", ...)` replacing `"ifconfig"`; line 77: `pub async fn iris_response_headers(...)` replacing `ifconfig_response_headers` |
| `src/lib.rs` | Line 146: `axum_mw::from_fn(middleware::iris_response_headers)`; `use iris::` imports in doc/tests; comment on line 144 |
| `src/config.rs` | `Environment::with_prefix("IRIS")` replacing `"IFCONFIG"` |
| `src/main.rs` | Log filter string: `"info,iris=debug,hyper=warn,h2=warn,mhost=warn"`; `use iris::` |
| `src/state.rs` | Any `Ifconfig` type alias → `IpInfo` |

**Files to rename:**

| From | To |
|------|----|
| `ifconfig.dev.toml` | `iris.dev.toml` |
| `ifconfig.example.toml` | `iris.example.toml` |

**Config file content updates:**
- All `IFCONFIG_` references in comments → `IRIS_`
- `project_name = "ifconfig-rs"` → `project_name = "iris"`
- `service_name = "ifconfig"` under `[telemetry]` → `service_name = "iris"`

**Phase complete when:** `cargo test --lib` passes (~168 unit tests), `cargo clippy` clean, `cargo build` succeeds.

---

### Phase 2: Frontend rename

**Commit message:** `refactor(iris): rename frontend SPA branding and types`

**Files to modify:**

| File | Change |
|------|--------|
| `frontend/package.json` | `"name": "iris-frontend"` |
| `frontend/src/lib/types.ts` | `Ifconfig` → `IpInfo` |
| `frontend/src/lib/api.ts` | `fetchIfconfig` → `fetchIpInfo`; `fetchIfconfigForIp` → `fetchIpInfoForIp` |
| `frontend/src/App.tsx` | Line 2: `import type { IpInfo, SiteMeta }`; line 3: `import { fetchIpInfo, fetchIpInfoForIp, fetchMeta }`; line 19: `createTheme('iris_theme', 'system')`; line 22: `createSignal<IpInfo \| null>`; line 34: `fetchIpInfoForIp`/`fetchIpInfo`; line 140: `<h1 class="logo">iris</h1>`; line 232: update footer text and GitHub link to `lukaspustina/iris`; line 238: update GitHub href |
| `frontend/src/components/IpDisplay.tsx` | Line 3: `import type { IpInfo, SiteMeta }`; line 7: `data: IpInfo`; line 66: `&ref=iris` replacing `&ref=ifconfig` |
| `frontend/src/components/InfoCards.tsx` | `Ifconfig` → `IpInfo` type import |
| `frontend/src/components/ApiExplorer.tsx` | Verify no `Ifconfig` type import exists (confirmed none in codebase); update any `ifconfig-rs` text references if present |
| `frontend/src/components/IpLookupForm.tsx` | Verify no `Ifconfig` type import exists (confirmed none in codebase); update any `ifconfig-rs` text references if present |
| `frontend/src/components/Faq.tsx` | GitHub links: `ifconfig-rs` → `iris` |
| `frontend/src/styles/global.css` | Line 9: update comment `/* ifconfig-rs-specific tokens */` → `/* iris-specific tokens */`. No CSS class or custom property names contain `ifconfig`; no functional CSS changes needed. |

**Phase complete when:** `cd frontend && npm run build` succeeds, dev server renders `iris` branding.

---

### Phase 3: Tests rename

**Commit message:** `refactor(iris): update test files for renamed crate and types`

**Files to modify:**

| File | Change |
|------|--------|
| `tests/ok_handlers.rs` | All `use ifconfig_rs::` → `use iris::`; `Config::load(Some("ifconfig.dev.toml"))` → `Config::load(Some("iris.dev.toml"))` (3 occurrences); `ifconfig_rs::build_app` → `iris::build_app`; line 1289: `json["components"]["schemas"]["IpInfo"]`; line 1290: `"Should have IpInfo schema"`; line 1598: update ignore comment |
| `tests/snapshots_test.rs` | Line 1: update comment; lines 12–13: `use iris::backend::{..., IpInfo, ...}`; `use iris::format::OutputFormat`; `make_test_ifconfig()` → `make_test_ip_info()`; all `Ifconfig {` → `IpInfo {`; function names on lines 15, 73, 129 |
| `tests/admin.rs` | `use ifconfig_rs::` → `use iris::`; `Config::load(Some("ifconfig.dev.toml"))` → `Config::load(Some("iris.dev.toml"))`; `ifconfig_rs::build_app` → `iris::build_app` |
| `tests/etag_last_modified.rs` | `use ifconfig_rs::` → `use iris::`; `Config::load(Some("ifconfig.dev.toml"))` → `Config::load(Some("iris.dev.toml"))`; `ifconfig_rs::build_app` → `iris::build_app` |
| `tests/error_handler.rs` | `use ifconfig_rs::` → `use iris::`; `Config::load(Some("ifconfig.dev.toml"))` → `Config::load(Some("iris.dev.toml"))`; `ifconfig_rs::build_app` → `iris::build_app` |
| `tests/rate_limit.rs` | `use ifconfig_rs::` → `use iris::`; `Config::load(Some("ifconfig.dev.toml"))` → `Config::load(Some("iris.dev.toml"))`; `ifconfig_rs::build_app` → `iris::build_app` |
| `tests/Dockerfile.tests` | `COPY iris.dev.toml iris.dev.toml` replacing `ifconfig.dev.toml` |
| `tests/e2e/package.json` | `"name": "iris"` |

**Snapshot regeneration:** Snapshot metadata headers contain `source: tests/snapshots_test.rs` and function names. After renaming `make_test_ifconfig()` to `make_test_ip_info()`, delete all files under `tests/snapshots/` and run `cargo test` to regenerate them. Wire-format field names are unchanged so snapshot content (JSON/YAML/TOML/CSV values) will be identical; only the metadata lines will differ.

**Phase complete when:** `cargo test` passes (all ~300 tests), new snapshots committed.

---

### Phase 4: Docker, CI/CD, Makefile

**Commit message:** `refactor(iris): rename Docker image, compose, CI workflows, and Makefile`

**Files to modify:**

| File | Change |
|------|--------|
| `Dockerfile` | Binary `COPY target/release/iris /iris/iris`; data image label `iris-data`; `RUN addgroup iris && adduser -G iris iris`; `WORKDIR /iris`; `ENV IRIS_SERVER__BIND=0.0.0.0:8000`; `COPY iris.example.toml iris.example.toml`; binary invocation `CMD ["iris", "iris.toml"]` |
| `docker-compose.yml` | Service name `iris`; `image: ghcr.io/netray-info/iris:latest`; container name `iris`; volume mounts use `iris` paths; env vars use `IRIS_` prefix |
| `.github/workflows/ci.yml` | `iris-data` image reference; any `ifconfig-rs` string in job names or comments |
| `.github/workflows/deploy.yml` | Webhook URL `deploy-iris` replacing `deploy-ifconfig-rs` |
| `.github/workflows/release.yml` | Image name if hardcoded (check for `ifconfig-rs` or `ifconfig_rs`) |
| `Makefile` | `APP := iris`; config file refs `iris.dev.toml` |
| `data/Makefile` | `ghcr.io/netray-info/iris-data` replacing `ghcr.io/netray-info/ifconfig-rs-data` (verify exact current value) |

**Phase complete when:** `make docker-build` succeeds, CI workflow YAML passes `yamllint`.

---

### Phase 5: Documentation (this repo)

**Commit message:** `refactor(iris): update all documentation from ifconfig-rs to iris`

**Files to modify:**

| File | Change |
|------|--------|
| `README.md` | Title, badges (`img.shields.io` image alt text and links), code examples (`./iris iris.dev.toml`), self-hosting instructions, GitHub URLs |
| `CLAUDE.md` | All ~30+ occurrences: crate name, binary name, config file names, env var prefix, module descriptions, env var examples, integration test comments |
| `docs/enrichment.md` | Title reference and any `ifconfig-rs` occurrences |
| `data/README.md` | Image name references |
| `CHANGELOG.md` | Add entry at top: `## Unreleased` → `## [Unreleased]` section with `### Changed` → `- Renamed project from ifconfig-rs to iris`; do not rewrite historical entries |

**Verification command:**
```sh
grep -ri "ifconfig" \
  --exclude-dir=node_modules \
  --exclude-dir=.git \
  --exclude="CHANGELOG.md" \
  --exclude="rename-ifconfig-rs-to-iris*.md" \
  .
```
Must return zero hits.

**Phase complete when:** Verification command returns zero hits.

---

### Phase 6: Meta repository (netray.info)

**Commit message:** `refactor(iris): update meta repo references from ifconfig-rs to iris`

**Files to modify:**

| File | Change |
|------|--------|
| `CLAUDE.md` | Suite architecture list (`iris`); inter-service deps (`iris`, `IRIS_`); gitignore list mention; config prefix; release checklist |
| `deploy/docker-compose.yml` | Service name `iris`; env vars for dependent services (`http://iris:8000`); dependency declarations |
| `deploy/README.md` | Text references |
| `site/index.html` | Marketing copy, GitHub link to `lukaspustina/iris` |
| `specs/rules/*.md` | Config prefix references if any |
| `specs/done/sdd/*.md` | Add at top of each file: `<!-- Renamed: ifconfig-rs to iris on 2026-04-11. This document is archived. -->` |

**Phase complete when:** `grep -ri "ifconfig" --exclude-dir=.git specs/done/` shows only files with the archive note; all other meta repo files are clean.

---

### Phase 7: Sibling repositories

Each repo gets its own `refactor:` commit.

**mhost-prism:**

| File | Change |
|------|--------|
| `prism.example.toml` | `http://iris:8000` |
| `prism.dev.toml` | `http://iris:8000`; rename config key `ifconfig_url` → `ip_url` |
| `src/api/meta.rs` | Struct field `ifconfig_url: String` → `ip_url: String` |
| `frontend/src/App.tsx` | Signal/type rename tracking `ip_url` field |
| `frontend/src/components/ResultsTable.tsx` | Prop rename if referencing `ifconfigUrl` |
| `CLAUDE.md`, docs | Text references |
| `CHANGELOG.md` | Add breaking change note: "`ifconfig_url` config key renamed to `ip_url`" |

**tlsight:**

| File | Change |
|------|--------|
| `tlsight.example.toml` | `http://iris:8000` |
| `CLAUDE.md`, `CHANGELOG.md`, docs | Text references |

**spectra:**

| File | Change |
|------|--------|
| `spectra.example.toml` | `http://iris:8000` |
| `spectra.dev.toml` | `http://iris:8000` (config key is already `ip_url` — no key rename needed) |
| `README.md`, `CHANGELOG.md`, `src/inspect/mod.rs` | Text references |

**beacon:**

| File | Change |
|------|--------|
| `beacon.example.toml` | `http://iris:8000` under `[enrichment] ip_url` |
| `beacon.dev.toml` | `http://iris:8000` |
| `CLAUDE.md` | Text references |

**lens:**

| File | Change |
|------|--------|
| `lens.dev.toml` | `http://iris:8000` (config key is already `ip_url` — no key rename needed) |
| `src/backends/ip.rs` | Comments referencing `ifconfig-rs` |
| `CLAUDE.md` | Text references |

**netray-common:**

| File | Change |
|------|--------|
| `src/enrichment.rs` | Doc comment updating `ifconfig-rs` to `iris` |

**netray-common-frontend:**

| File | Change |
|------|--------|
| `CLAUDE.md`, `README.md` | Consumer list: `ifconfig-rs` → `iris` |

**Phase complete when:** Each repo builds and tests clean. No `grep -ri ifconfig` hits outside changelogs.

---

### Phase 8: argus-oci production deployment

**Commit message:** `refactor(iris): rename Ansible role and all deployment config from ifconfig-rs`

This phase is operationally sensitive — it changes the live Docker service name.

**Rename:**
- `roles/ifconfig_rs/` → `roles/iris/`
- `roles/iris/templates/ifconfig.toml.j2` → `roles/iris/templates/iris.toml.j2`

**Files to modify:**

| File | Change |
|------|--------|
| `ansible/playbooks/site.yml` | `{role: iris, tags: [apps, iris]}` |
| `ansible/inventory/group_vars/all/main.yml` | All `ifconfig_rs_*` vars → `iris_*`; webhook name `deploy-iris` |
| `roles/iris/tasks/main.yml` | Task names; template path `iris.toml.j2`; all `ifconfig_rs` → `iris` |
| `roles/iris/handlers/main.yml` | Handler name |
| `roles/iris/templates/docker-compose.yml.j2` | Service name `iris`; image `ghcr.io/netray-info/iris`; container name `iris`; volume mount path; Traefik labels (router name and service name); promtail label |
| `roles/iris/templates/iris.toml.j2` | (previously `ifconfig.toml.j2`) Variable refs; `service_name = "iris"` |
| `roles/prism/templates/prism.toml.j2` | `http://iris:8000`; `iris_domain` var replacing `ifconfig_rs_domain` |
| `roles/tlsight/templates/tlsight.toml.j2` | `http://iris:8000`; `iris_domain` var |
| `roles/lens/templates/lens.toml.j2` | `http://iris:8000` |
| `roles/spectra/templates/spectra.toml.j2` | `http://iris:8000` |
| `roles/beacon/templates/beacon.toml.j2` | `http://iris:8000` |
| `roles/monitoring/templates/otel-collector-config.yml.j2` | Job name `iris`; scrape target `iris:9090` |
| Grafana dashboard JSON(s) | Metric namespace `ifconfig` → `iris` in all panel queries and alert rules |
| `CLAUDE.md`, `README.md`, `specs/SDD.md` | Text references |

**Deployment procedure:**
1. Push new `iris` Docker image to GHCR (done via Phase 4 CI)
2. Run full Ansible playbook — all services redeploy simultaneously (iris gets new name, siblings get updated enrichment URL)
3. Verify all services healthy: `curl https://ip.netray.info/health` and equivalent for each sibling
4. Verify metrics appear under `iris_*` namespace in Prometheus/Grafana

**Rollback:** If the `iris` container fails its health check, revert the Docker image tag in group vars to the last known-good tag and re-run the playbook. Expected restart window is under 30 seconds.

**Phase complete when:** `ansible-playbook site.yml --check` shows only expected changes; production deploy succeeds; all `/health` and `/ready` endpoints return 200; Grafana dashboards show data under new `iris` namespace.

---

### Phase 9: GitHub repository rename + webhook update

**Actions (manual — no commit):**

1. Rename `lukaspustina/ifconfig-rs` → `lukaspustina/iris` via GitHub Settings > General
2. Update deploy webhook URL in argus-oci webhook config from `deploy-ifconfig-rs` to `deploy-iris` (already done in Phase 8 group vars)
3. Update local git remote: `git remote set-url origin git@github.com:lukaspustina/iris.git`
4. Update any CI secrets or environment variables referencing the old repo name
5. Verify CI triggers on next push to the renamed repo

**Phase complete when:** CI runs green on `lukaspustina/iris`; deploy webhook fires successfully on tag push.

---

## Test Scenarios

**GIVEN** the crate is renamed to `iris`
**WHEN** `cargo test` runs
**THEN** all ~300 tests pass with the new struct/function names and `iris.dev.toml` config path

**GIVEN** the env prefix is `IRIS_`
**WHEN** `IRIS_SERVER__BIND=0.0.0.0:9999 ./iris iris.dev.toml` is run
**THEN** the server binds to port 9999

**GIVEN** the frontend uses `iris_theme` as the localStorage key
**WHEN** a user visits the site with an existing `ifconfig_theme` preference
**THEN** the theme resets to system default (acceptable one-time break; no migration code required)

**GIVEN** the Docker service is renamed to `iris`
**WHEN** prism calls `http://iris:8000/json?ip=1.1.1.1`
**THEN** it receives a valid `IpInfo` JSON response

**GIVEN** the Ansible role is renamed to `iris`
**WHEN** `ansible-playbook site.yml` runs
**THEN** the old `ifconfig-rs` container is replaced by the `iris` container with no downtime beyond the restart window (~30 s)

**GIVEN** the GitHub repo is renamed
**WHEN** someone clones from `github.com/lukaspustina/ifconfig-rs`
**THEN** GitHub redirects to `github.com/lukaspustina/iris`

**GIVEN** Prometheus metrics use namespace `iris`
**WHEN** the service starts after Phase 8 deploy
**THEN** `iris_http_requests_total` and `iris_http_request_duration_seconds` appear in `/metrics` output

**GIVEN** the OpenAPI schema is regenerated after struct rename
**WHEN** `GET /api-docs/openapi.json` is called
**THEN** `components.schemas` contains `IpInfo` (not `Ifconfig`)

**GIVEN** snapshot files are deleted and `cargo test` is run
**WHEN** insta generates new snapshots
**THEN** snapshot content (JSON/YAML/TOML/CSV values) is identical to the previous snapshots; only metadata headers differ

## Decision Log

| Decision | Alternatives considered | Why rejected |
|----------|------------------------|--------------|
| Rename `Ifconfig` struct to `IpInfo` | Keep `Ifconfig` (internal only); rename to `Iris`; rename to `Enrichment` | `Ifconfig` leaks the old name into OpenAPI schema and TypeScript types. `Iris` couples the type to the brand. `Enrichment` is too vague. `IpInfo` describes what the struct contains. |
| Clean cut, no transition period | Dual-name period with aliases | Adds complexity for zero benefit — no external crate consumers, no public stable API contract beyond the JSON wire format (which doesn't change). Per project convention: no backwards-compat shims. |
| Keep `ip.netray.info` DNS | Change to `iris.netray.info` | `ip.` is a service descriptor, not a project name. Users bookmark and script against it. Changing it breaks existing integrations for no user benefit. |
| Rename mhost-prism's `ifconfig_url` to `ip_url` | Keep `ifconfig_url` with just the value changed | Other repos already use `ip_url`. Aligning prism eliminates the last project-name-coupled config key across the ecosystem. Breaking change is acceptable — very few self-hosters exist. |
| Single coordinated Ansible deploy | Rolling deploy with Docker network aliases | Ansible already deploys all services atomically. Network aliases add complexity for a one-time rename event. |
| Rename Prometheus namespace to `iris` and update dashboards in same deploy (option a) | Emit metrics under both names temporarily (option b) | Option (b) adds tech debt and a second cleanup deploy. Option (a) is one atomic change. Grafana dashboard updates are straightforward find-and-replace on metric names. |
| Rename mhost-prism `ifconfig_url` → `ip_url` and document in CHANGELOG | Accept both keys with deprecation warning | Deprecation adds code complexity. Very few self-hosters exist; a clean CHANGELOG entry is sufficient. |

## Open Decisions

None.

## Out of Scope

- **API wire format changes**: JSON field names (`ip`, `location`, `network`, etc.) are unchanged. No client-side breakage.
- **Endpoint path changes**: All paths (`/`, `/ip`, `/json`, `/batch`, etc.) remain identical.
- **Feature changes**: No new features, no removed features. Pure rename.
- **DNS changes**: `ip.netray.info` is not changing.
- **crates.io**: The crate is not published; no crate rename needed.
- **Historical git rewriting**: Commit history retains `ifconfig` references. Only CHANGELOG gets a rename note.
- **Completed SDD rewriting**: Files in `specs/done/` get a header note but are not rewritten.
- **Theme migration**: Existing `ifconfig_theme` localStorage values are not migrated; users get a one-time reset to system default.
