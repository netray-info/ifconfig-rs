# SDD: Rename ifconfig-rs to iris

**Status:** Draft
**Created:** 2026-04-11

## Overview

Rename the ifconfig-rs project to **iris** across the entire netray.info ecosystem. This aligns the IP enrichment service with the suite's optics/light naming theme (prism, sight, spectra, beacon, lens) and replaces the generic "ifconfig" name inherited from the Unix utility. The rename touches the Rust crate, binary, frontend SPA, Docker images, CI/CD, deployment infrastructure (argus-oci), and cross-references in all six sibling repositories.

## Context & Constraints

- **Stack**: Rust (Axum 0.8, tokio), SolidJS 1.9 frontend, Vite 6, Docker multi-stage builds
- **DNS stays**: `ip.netray.info` is a service domain, not a project name -- it does not change
- **GitHub redirect**: Renaming `lukaspustina/ifconfig-rs` to `lukaspustina/iris` auto-creates a redirect; existing clones continue to push/pull
- **GHCR images**: Old image tags remain accessible; new pushes go to `ghcr.io/netray-info/iris`
- **Coordinated deploy**: Changing the Docker service/container name means all sibling services referencing `http://ifconfig-rs:8000` must be redeployed in the same Ansible run
- **No backwards compat shims**: Clean cut, no transition period (per project conventions)
- **Conventional commits**: Use `refactor:` prefix for rename commits within each repo
- **Scoped changes**: Rename-only commits -- no functional changes mixed in

## Requirements

1. The Rust crate shall be named `iris` with binary name `iris`.
2. The central data struct shall be renamed from `Ifconfig` to `IpInfo` (neutral name reflecting what it contains, not the project name -- avoids coupling the public API type to the project brand).
3. The environment variable prefix shall change from `IFCONFIG_` to `IRIS_`.
4. Config files shall be renamed: `iris.dev.toml`, `iris.example.toml`.
5. The Docker image shall be `ghcr.io/netray-info/iris` (and `iris-data` for the data image).
6. The Ansible role shall be renamed from `ifconfig_rs` to `iris` with all variable prefixes updated.
7. All sibling services shall reference the new Docker hostname `iris` (not `ifconfig-rs`) for internal enrichment calls.
8. The frontend SPA shall display "iris" as the logo text and use `iris_theme` as the localStorage key.
9. The OpenAPI spec title shall be "iris".
10. The Prometheus metrics namespace shall change from `"ifconfig"` to `"iris"`.
11. The log filter shall use `iris=debug` instead of `ifconfig_rs=debug`.
12. The deploy webhook path shall change from `deploy-ifconfig-rs` to `deploy-iris`.
13. The GitHub repository shall be renamed from `lukaspustina/ifconfig-rs` to `lukaspustina/iris`.
14. All documentation (README, CLAUDE.md, enrichment.md, data/README.md) shall reference "iris" consistently.
15. Cross-repo documentation and config comments referencing "ifconfig-rs" shall be updated in all sibling repos and the meta repo.

## Architecture

No architectural changes. The service, its API surface, endpoints, and wire format remain identical. This is a pure naming change across:

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

## Implementation Phases

### Phase 1: ifconfig-rs core rename (this repo)

Rename the Rust crate, binary, structs, functions, config prefix, and all internal references.

**Files to modify:**

| File | Changes |
|------|---------|
| `Cargo.toml` | name, binary name, keywords, include paths |
| `src/backend/mod.rs` | `Ifconfig` → `IpInfo`, `IfconfigParam` → `IpInfoParam`, `get_ifconfig()` → `get_ip_info()` |
| `src/handlers.rs` | `make_ifconfig()` → `make_ip_info()`, `make_ifconfig_lang()` → `make_ip_info_lang()`, all `&Ifconfig` params |
| `src/routes.rs` | OpenAPI title, all `Ifconfig` type refs, `ifconfig` variable names, dispatch functions |
| `src/middleware.rs` | `http_metrics("iris", ...)`, `ifconfig_response_headers` → `iris_response_headers` |
| `src/config.rs` | `Environment::with_prefix("IRIS")` |
| `src/main.rs` | `use iris::`, log filter `iris=debug` |
| `src/lib.rs` | middleware function reference |
| `src/state.rs` | `Ifconfig` → `IpInfo` in type alias |
| `src/error.rs` | No changes expected (uses generic types) |

**Files to rename:**

| From | To |
|------|-----|
| `ifconfig.dev.toml` | `iris.dev.toml` |
| `ifconfig.example.toml` | `iris.example.toml` |

**Config file content updates:**
- Comments referencing `IFCONFIG_` → `IRIS_`
- `project_name = "ifconfig-rs"` → `project_name = "iris"`

**Phase complete when:** `cargo test --lib` passes, `cargo clippy` clean, `cargo build` succeeds with renamed config files.

### Phase 2: Frontend rename (this repo)

Update the SolidJS SPA to reflect the new name.

| File | Changes |
|------|---------|
| `frontend/package.json` | `"name": "iris-frontend"` |
| `frontend/src/lib/types.ts` | `Ifconfig` → `IpInfo` |
| `frontend/src/lib/api.ts` | `fetchIfconfig` → `fetchIpInfo`, `fetchIfconfigForIp` → `fetchIpInfoForIp` |
| `frontend/src/App.tsx` | Import renames, `createTheme('iris_theme', ...)`, `<h1>iris</h1>`, GitHub links, footer text |
| `frontend/src/components/InfoCards.tsx` | `Ifconfig` → `IpInfo` type import |
| `frontend/src/components/IpDisplay.tsx` | `Ifconfig` → `IpInfo` type import, `&ref=iris` query param |
| `frontend/src/components/Faq.tsx` | GitHub links `ifconfig-rs` → `iris` |
| `frontend/src/styles/global.css` | Comment update |

**Phase complete when:** `cd frontend && npm run build` succeeds, dev server renders correctly with "iris" branding.

### Phase 3: Tests rename (this repo)

Update all test files to use new names.

| File | Changes |
|------|---------|
| `tests/ok_handlers.rs` | `Config::load(Some("iris.dev.toml"))`, OpenAPI schema assertion `IpInfo` |
| `tests/snapshots_test.rs` | `make_test_ip_info()` functions, `IpInfo` struct refs, `use iris::` |
| `tests/admin.rs` | Config path |
| `tests/etag_last_modified.rs` | Config path |
| `tests/error_handler.rs` | Config path |
| `tests/rate_limit.rs` | Config path |
| `tests/Dockerfile.tests` | `COPY iris.dev.toml iris.dev.toml` |
| `tests/e2e/package.json` | `"name": "iris"` |

**Phase complete when:** `cargo test` passes (all ~300 tests), snapshot tests updated.

### Phase 4: Docker, CI/CD, Makefile (this repo)

| File | Changes |
|------|---------|
| `Dockerfile` | Binary name `iris`, data image `iris-data`, user/group `iris`, workdir `/iris`, config names, `ENV IRIS_SERVER__BIND` |
| `docker-compose.yml` | Service name, image name, volume paths, env vars |
| `.github/workflows/ci.yml` | `iris-data` image reference |
| `.github/workflows/deploy.yml` | Webhook URL `deploy-iris` |
| `.github/workflows/release.yml` | Image name (if hardcoded beyond `github.repository`) |
| `Makefile` | `APP := iris`, config file refs |
| `data/Makefile` | `ghcr.io/netray-info/iris-data` |

**Phase complete when:** `make docker-build` succeeds, CI workflow YAML is valid.

### Phase 5: Documentation (this repo)

| File | Changes |
|------|---------|
| `README.md` | Title, badges, image alt text, code examples, self-hosting instructions, GitHub URLs |
| `CLAUDE.md` | All references (~30+), config examples, module descriptions, env var examples |
| `docs/enrichment.md` | Title reference |
| `data/README.md` | Image name references |
| `CHANGELOG.md` | Add rename entry at top; do not rewrite history |

**Phase complete when:** `grep -ri ifconfig` returns zero hits outside of `CHANGELOG.md` historical entries, `node_modules/`, and `.git/`.

### Phase 6: Meta repository (netray.info)

| File | Changes |
|------|---------|
| `CLAUDE.md` | Suite architecture list, inter-service deps, gitignore list, config prefix, release checklist |
| `deploy/docker-compose.yml` | Service name `iris`, env vars for dependent services (`http://iris:8000`), dependency declarations |
| `deploy/README.md` | Text references |
| `site/index.html` | Marketing copy, GitHub link |
| `specs/rules/*.md` | Config prefix references (if any) |
| `specs/done/sdd/*.md` | Historical references -- add a note at top of each, do not rewrite |

**Phase complete when:** `grep -ri ifconfig` in meta repo returns only historical SDD references with a note.

### Phase 7: Sibling repositories (5 services + 2 libraries)

Each repo gets a scoped commit updating references:

**mhost-prism:**
- `prism.example.toml`: `http://iris:8000`
- `prism.dev.toml`: rename `ifconfig_url` config key to `ip_url` (align with other repos)
- `src/api/meta.rs`: struct field rename `ifconfig_url` → `ip_url`
- `frontend/src/App.tsx`: signal/type rename
- `frontend/src/components/ResultsTable.tsx`: prop rename
- `CLAUDE.md`, docs: text references

**tlsight:**
- `tlsight.example.toml`: `http://iris:8000`
- `CLAUDE.md`, `CHANGELOG.md`, docs: text references

**spectra:**
- `spectra.example.toml`: `http://iris:8000`
- `spectra.dev.toml`: already uses `ip_url` (no key rename needed)
- `README.md`, `CHANGELOG.md`, `src/inspect/mod.rs`: text references

**lens:**
- `lens.dev.toml`: already uses `ip_url` (no key rename needed)
- `src/backends/ip.rs`: comments referencing "ifconfig-rs"
- `CLAUDE.md`: text references

**netray-common:**
- `src/enrichment.rs`: doc comment

**netray-common-frontend:**
- `CLAUDE.md`, `README.md`: consumer list

**Phase complete when:** Each repo builds and tests clean. No `grep -ri ifconfig` hits outside changelogs.

### Phase 8: argus-oci production deployment

This is the most operationally sensitive phase -- it changes Docker service names on the live server.

**Rename the Ansible role directory:**
- `roles/ifconfig_rs/` → `roles/iris/`

**Update files:**

| File | Changes |
|------|---------|
| `ansible/playbooks/site.yml` | `{role: iris, tags: [apps, iris]}` |
| `ansible/inventory/group_vars/all/main.yml` | All `ifconfig_rs_*` vars → `iris_*`, webhook name |
| `roles/iris/tasks/main.yml` | Task names, paths, handlers |
| `roles/iris/handlers/main.yml` | Handler name |
| `roles/iris/templates/docker-compose.yml.j2` | Service name, image, container name, volume mount, Traefik labels (router + service names), promtail label |
| `roles/iris/templates/ifconfig.toml.j2` → `iris.toml.j2` | Variable refs, `service_name = "iris"` |
| `roles/prism/templates/prism.toml.j2` | `http://iris:8000`, `iris_domain` var |
| `roles/tlsight/templates/tlsight.toml.j2` | `http://iris:8000`, `iris_domain` var |
| `roles/lens/templates/lens.toml.j2` | `http://iris:8000` |
| `roles/spectra/templates/spectra.toml.j2` | `http://iris:8000` |
| `roles/monitoring/templates/otel-collector-config.yml.j2` | job name, scrape target |
| `CLAUDE.md`, `README.md`, `specs/SDD.md` | Text references |

**Deployment procedure:**
1. Push new `iris` Docker image to GHCR
2. Run full Ansible playbook -- all services redeploy simultaneously (iris gets new name, siblings get updated enrichment URL)
3. Verify all services healthy via `/health` endpoints

**Phase complete when:** `ansible-playbook site.yml --check` shows expected changes, production deploy succeeds, all `/health` and `/ready` endpoints return 200.

### Phase 9: GitHub repository rename + webhook update

1. Rename `lukaspustina/ifconfig-rs` → `lukaspustina/iris` via GitHub Settings
2. Update deploy webhook URL in argus-oci webhook config
3. Update local git remotes: `git remote set-url origin git@github.com:lukaspustina/iris.git`
4. Verify CI triggers on next push

**Phase complete when:** CI runs green on the renamed repo, deploy webhook fires successfully.

## Test Scenarios

**GIVEN** the crate is renamed to `iris`
**WHEN** `cargo test` runs
**THEN** all ~300 tests pass with the new struct/function names and config paths

**GIVEN** the env prefix is `IRIS_`
**WHEN** `IRIS_SERVER__BIND=0.0.0.0:9999 ./iris iris.dev.toml` is run
**THEN** the server binds to port 9999

**GIVEN** the frontend uses `iris_theme` localStorage key
**WHEN** a user visits the site with an existing `ifconfig_theme` preference
**THEN** the theme resets to system default (acceptable one-time break)

**GIVEN** the Docker service is renamed to `iris`
**WHEN** prism calls `http://iris:8000/json?ip=1.1.1.1`
**THEN** it receives a valid enrichment response

**GIVEN** the Ansible role is renamed to `iris`
**WHEN** `ansible-playbook site.yml` runs
**THEN** the old `ifconfig-rs` container is replaced by the `iris` container with no downtime beyond the restart window

**GIVEN** the GitHub repo is renamed
**WHEN** someone clones from `lukaspustina/ifconfig-rs`
**THEN** GitHub redirects to `lukaspustina/iris`

## Decision Log

| Decision | Alternatives considered | Why rejected |
|----------|------------------------|--------------|
| Rename `Ifconfig` struct to `IpInfo` | Keep `Ifconfig` (internal only), rename to `Iris`, rename to `Enrichment` | `Ifconfig` leaks the old name into OpenAPI schema and TypeScript types. `Iris` couples the type to the brand. `Enrichment` is too vague. `IpInfo` describes what the struct contains. |
| Clean cut, no transition period | Dual-name period with aliases | Adds complexity for zero benefit -- no external crate consumers, no public stable API contract beyond the JSON wire format (which doesn't change). Per project convention: no backwards-compat shims. |
| Keep `ip.netray.info` DNS | Change to `iris.netray.info` | `ip.` is a service descriptor, not a project name. Users bookmark and script against it. Changing it breaks existing integrations for no user benefit. |
| Rename mhost-prism's `ifconfig_url` to `ip_url` | Keep `ifconfig_url` with just the value changed | Other repos already use `ip_url`. Aligning prism eliminates the last project-name-coupled config key across the ecosystem. This is a breaking config change for prism users but the key was always oddly named. |
| Single coordinated Ansible deploy | Rolling deploy with Docker network aliases | Ansible already deploys all services atomically. Network aliases add complexity for a one-time rename event. |

## Open Decisions

1. **Prometheus metric history**: Renaming the metrics namespace from `ifconfig` to `iris` breaks existing Grafana dashboards and alert rules. **Options:** (a) Rename and update dashboards simultaneously, (b) emit metrics under both names temporarily. **Impact:** Option (a) is cleaner but requires dashboard updates in the same deploy. Option (b) avoids dashboard urgency but adds tech debt.

2. **mhost-prism `ifconfig_url` config key**: This is a user-visible breaking change for anyone running mhost-prism with a custom config. **Options:** (a) Rename to `ip_url` and document the break in CHANGELOG, (b) accept both keys with deprecation warning. **Impact:** Very few self-hosters exist; option (a) is simpler.

## Out of Scope

- **API wire format changes**: JSON field names (`ip`, `location`, `network`, etc.) are unchanged. No client-side breakage.
- **Endpoint path changes**: All paths (`/`, `/ip`, `/json`, `/batch`, etc.) remain identical.
- **Feature changes**: No new features, no removed features. Pure rename.
- **DNS changes**: `ip.netray.info` is not changing.
- **crates.io**: The crate is not published; no crate rename needed.
- **Historical git rewriting**: Commit history retains "ifconfig" references. Only CHANGELOG gets a rename note.
- **Completed SDD rewriting**: Files in `specs/done/` get a header note but are not rewritten.
