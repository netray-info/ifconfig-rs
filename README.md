<div align="center">
  <h1>ifconfig-rs</h1>
  <p><strong>IP enrichment API. Your IP, any IP. From the terminal or the browser.</strong></p>
  <p>
    <a href="https://ip.netray.info"><img src="https://img.shields.io/badge/ip.netray.info-live-brightgreen.svg" alt="Live at ip.netray.info" /></a>
    <a href="https://github.com/lukaspustina/ifconfig-rs/actions/workflows/ci.yml"><img src="https://github.com/lukaspustina/ifconfig-rs/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
    <a href="https://github.com/lukaspustina/ifconfig-rs/releases"><img src="https://img.shields.io/github/release/lukaspustina/ifconfig-rs.svg" alt="GitHub release" /></a>
    <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" />
  </p>
  <p>
    <a href="#quick-start">Quick Start</a> |
    <a href="#api-reference">API Reference</a> |
    <a href="#output-formats">Formats</a> |
    <a href="#self-hosting">Self-Hosting</a> |
    <a href="#configuration">Configuration</a> |
    <a href="docs/enrichment.md">Enrichment Pipeline</a>
  </p>
</div>

Part of the [netray.info](https://netray.info) network intelligence suite.

---

_ifconfig-rs_ is a fast, self-hostable IP enrichment API written in [Rust](https://www.rust-lang.org), powered by [Axum](https://github.com/tokio-rs/axum), with a [SolidJS](https://www.solidjs.com) SPA. Beyond "what's my IP", it classifies IPs by network type (cloud, VPN, Tor, bot, hosting, residential), detects cloud providers (AWS, GCP, Azure, Cloudflare), and supports batch lookups and arbitrary IP queries. It serves a clean browser UI to humans and plain text to scripts — no configuration needed on the client side. Currently powering [ip.netray.info](https://ip.netray.info).

```sh
$ curl ip.netray.info
203.0.113.42
```

That's it. One command. Your IP. No noise.

---

## Quick Start

```sh
# Your public IP address — the most common use case
curl ip.netray.info

# Works great with wget and httpie too
wget -qO- ip.netray.info
http ip.netray.info

# Want more than just the IP?
curl ip.netray.info/all

# Everything as JSON — pipe it anywhere
curl ip.netray.info/json

# Just your IPv4 address (useful from dual-stack machines)
curl -4 ip.netray.info/ipv4

# Just your IPv6 address
curl -6 ip.netray.info/ipv6

# Where are you connecting from?
curl ip.netray.info/location

# Which ISP and ASN?
curl ip.netray.info/isp

# Reverse DNS hostname
curl ip.netray.info/host

# What headers did you send?
curl ip.netray.info/headers

# Your IP as a CIDR prefix (handy for Terraform/Ansible)
curl ip.netray.info/ip/cidr

# Look up any IP address
curl 'ip.netray.info/all/json?ip=8.8.8.8'

# Health check (liveness)
curl ip.netray.info/health

# Readiness probe (checks GeoIP databases are loaded)
curl ip.netray.info/ready
```

**Pro tip:** Set up a shell alias and never think about it again:

```sh
alias myip="curl -s ip.netray.info"
```

---

## Why ifconfig-rs?

There are plenty of "what's my IP" services. Here's why this one is worth self-hosting:

| | ifconfig.co | wtfismyip | ipinfo.io | **ifconfig-rs** |
|---|:---:|:---:|:---:|:---:|
| Single static binary | | | | **yes** |
| No rate-limit surprises | limited | limited | limited | **configurable** |
| JSON, YAML, TOML, CSV | JSON only | JSON only | JSON only | **all four** |
| Arbitrary IP lookup (`?ip=`) | | | yes | **yes** |
| Batch lookup (`POST /batch`) | | | paid | **yes** |
| Field filtering (`?fields=`) | | | yes | **yes** |
| Network classification (cloud/VPN/Tor/bot) | | | partial | **yes** |
| Cloud provider detection (AWS/GCP/Azure) | | | | **yes** |
| OpenAPI spec | | | yes | **yes** |
| Interactive API docs (Scalar) | | | yes | **yes** |
| Structured JSON error responses | | | yes | **yes** |
| CLI auto-detection (plain text) | yes | yes | | **yes** |
| SPA with interactive API explorer | | | | **yes** |
| Dark / light / system theme | | | | **yes** |
| Trusted-proxy / XFF support | | yes | | **yes** |
| Prometheus metrics | | | | **yes** |
| Response compression (gzip) | | | | **yes** |
| Request ID for log correlation | | | | **yes** |
| Zero external runtime dependencies | | | | **yes** |
| Self-host in 5 minutes | | | | **yes** |

---

## API Reference

Every endpoint accepts a format suffix or an `Accept` header — see [Output Formats](#output-formats).

| Endpoint | Returns | Example |
|----------|---------|---------|
| `/` | Full info (SPA for browsers, plain IP for CLIs) | `curl ip.netray.info` |
| `/ip` | Your IP address | `curl ip.netray.info/ip` |
| `/ip/cidr` | Your IP as a CIDR prefix (`/32` or `/128`) | `curl ip.netray.info/ip/cidr` |
| `/ipv4` | Your IPv4 address | `curl -4 ip.netray.info/ipv4` |
| `/ipv6` | Your IPv6 address | `curl -6 ip.netray.info/ipv6` |
| `/tcp` | Your IP and source port | `curl ip.netray.info/tcp` |
| `/host` | Reverse DNS hostname | `curl ip.netray.info/host` |
| `/location` | City, region, country, coordinates | `curl ip.netray.info/location` |
| `/isp` | ASN number and organisation name | `curl ip.netray.info/isp` |
| `/network` | Network classification (type, provider, flags) | `curl ip.netray.info/network` |
| `/user_agent` | Parsed browser / OS / device info | `curl ip.netray.info/user_agent` |
| `/all` | Everything at once | `curl ip.netray.info/all` |
| `/headers` | Your raw request headers (includes `x_forwarded_for_chain`) | `curl ip.netray.info/headers` |
| `GET /asn/{number}` | ASN lookup by number (org, category, network_role, is_anycast) | `curl ip.netray.info/asn/15169` |
| `GET /range?cidr=<prefix>` | Classification of the given CIDR network address | `curl 'ip.netray.info/range?cidr=8.8.8.0/24'` |
| `POST /diff` | Compare enrichment between two IPs (body: `{"a":"<ip>","b":"<ip>"}`) | `curl -X POST -d '{"a":"8.8.8.8","b":"1.1.1.1"}' ip.netray.info/diff` |
| `POST /batch` | Bulk IP lookup (JSON array input) | `curl -X POST -d '["8.8.8.8"]' ip.netray.info/batch` |
| `/meta` | Site metadata (name, version, batch config, rate limit settings, loaded data sources) — JSON only, used by the SPA | `curl ip.netray.info/meta` |
| `/api-docs/openapi.json` | OpenAPI 3.1 specification | `curl ip.netray.info/api-docs/openapi.json` |
| `/docs` | Interactive API reference (Scalar UI) | Open in browser |
| `/health` | Liveness probe | `curl ip.netray.info/health` |
| `/ready` | Readiness probe (checks GeoIP DBs are loaded; body includes `warnings` for missing optional sources) | `curl ip.netray.info/ready` |

### Content Negotiation

ifconfig-rs figures out what you want automatically — no flags needed:

1. **Format suffix** — `/ip/json`, `/location/yaml`, `/all/csv` — highest priority
2. **`?format=` parameter** — `?format=json`, `?format=yaml`, etc. — alias for suffix, same priority
3. **CLI detection** — `curl`, `wget`, `httpie`, `python-httpx`, `python-requests` with `Accept: */*` get plain text
4. **`Accept` header** — standard content negotiation
5. **Default** — browsers get the SPA

### Locale-Aware Names (`?lang=`)

Pass a BCP-47 language tag to receive city and country names in your preferred language (when available in the GeoIP database):

```sh
curl 'ip.netray.info/location/json?lang=de'
curl 'ip.netray.info/all/json?ip=8.8.8.8&lang=ja'
```

### Error Responses

All error responses are structured JSON:

```json
{"error": {"code": "INVALID_IP", "message": "private/loopback IP not allowed"}}
```

This applies to 400 Bad Request, 404 Not Found, 429 Too Many Requests, and 500 Internal Server Error.

### Arbitrary IP Lookup (`?ip=`)

Look up any public IP address instead of your own:

```sh
# Full enrichment of 8.8.8.8
curl ip.netray.info/all/json?ip=8.8.8.8

# Location of a specific IP
curl ip.netray.info/location?ip=8.8.8.8

# Network classification
curl 'ip.netray.info/network/json?ip=1.1.1.1'

# Skip reverse DNS for this lookup (PTR is performed by default for ?ip= queries)
curl 'ip.netray.info/all/json?ip=8.8.8.8&dns=false'
```

For `?ip=` lookups, `tcp` (source port) is omitted — it isn't meaningful for arbitrary IPs. Reverse DNS (`ip.hostname`) is performed by default; use `&dns=false` to skip PTR lookups.

Private, loopback, and link-local addresses are rejected (400 Bad Request).

### Field Filtering (`?fields=`)

Return only the fields you need:

```sh
# Just IP and location
curl 'ip.netray.info/all/json?fields=ip,location'

# Combine with ?ip= for targeted lookups
curl 'ip.netray.info/all/json?ip=8.8.8.8&fields=ip,isp,network'
```

### Batch Lookup (`POST /batch`)

Look up multiple IPs in a single request (must be enabled in config):

```sh
# JSON output (default)
curl -X POST -H 'Content-Type: application/json' \
  -d '["8.8.8.8", "1.1.1.1"]' \
  ip.netray.info/batch

# CSV output — one row per IP, great for spreadsheets and SIEM tools
curl -X POST -H 'Content-Type: application/json' \
  -d '["8.8.8.8", "1.1.1.1"]' \
  ip.netray.info/batch/csv

# With field filtering
curl -X POST -H 'Content-Type: application/json' \
  -d '["8.8.8.8", "1.1.1.1"]' \
  'ip.netray.info/batch?fields=ip,location'
```

- Max 100 IPs per request (configurable)
- N IPs consume N rate-limit tokens
- Invalid/private IPs return per-IP errors inline, not a global failure
- Disabled by default — enable with `batch.enabled = true` in config

---

## Output Formats

Append a format to any endpoint, or use an `Accept` header:

```sh
# Format suffix (easiest)
curl ip.netray.info/all/json
curl ip.netray.info/all/yaml
curl ip.netray.info/all/toml
curl ip.netray.info/all/csv

# Or at the root
curl ip.netray.info/json

# Or via Accept header
curl -H "Accept: application/json" ip.netray.info/all
curl -H "Accept: application/yaml" ip.netray.info/all
curl -H "Accept: application/toml" ip.netray.info/all
curl -H "Accept: text/csv"         ip.netray.info/all
```

| Format | Suffix | Content-Type |
|--------|--------|--------------|
| Plain text | *(CLI auto-detect)* | `text/plain` |
| JSON | `/json` | `application/json` |
| YAML | `/yaml` | `application/yaml` |
| TOML | `/toml` | `application/toml` |
| CSV | `/csv` | `text/csv` |

### Sample JSON Response (`/all/json`)

> **Note:** For `?ip=` queries, `tcp` and `host` are `null` (source port and reverse DNS aren't meaningful for arbitrary IPs).

```json
{
  "host": { "name": "ptr-203-0-113-42.example.net" },
  "ip": { "addr": "203.0.113.42", "version": "4" },
  "tcp": { "port": 54321 },
  "location": {
    "city": "Berlin",
    "country": "Germany",
    "country_iso": "DE",
    "latitude": 52.52,
    "longitude": 13.405,
    "timezone": "Europe/Berlin",
    "continent": "Europe",
    "continent_code": "EU",
    "accuracy_radius_km": 100
  },
  "isp": { "name": "Example ISP GmbH", "asn": 1234 },
  "network": {
    "asn": 1234,
    "org": "Example ISP GmbH",
    "prefix": "203.0.113.0/24",
    "asn_category": "isp",
    "network_role": null,
    "type": "residential",
    "infra_type": "residential",
    "is_internal": false,
    "is_datacenter": false,
    "is_vpn": false,
    "is_tor": false,
    "is_bot": false,
    "is_c2": false,
    "is_spamhaus": false,
    "is_anycast": false,
    "is_cins": false,
    "iana_label": null,
    "cloud": null,
    "vpn": null,
    "bot": null
  },
  "user_agent": {
    "device": { "family": "Other", "brand": null, "model": null },
    "os": { "family": "Other", "major": null, "minor": null, "patch": null, "patch_minor": null, "version": "" },
    "browser": { "family": "curl", "major": "8", "minor": "6", "patch": "0", "version": "8.6.0" }
  },
  "user_agent_header": "curl/8.6.0"
}
```

### CI / Pipeline Integration

Use in GitHub Actions or any CI system to validate IP reputation:

```yaml
# Check IP reputation
- run: |
    curl -sf "https://ip.netray.info/network/json?ip=$IP" \
      | jq -e '.type == "residential"'

# Verify an IP is not on threat lists
- run: |
    curl -sf "https://ip.netray.info/network/json?ip=$IP" \
      | jq -e '.is_c2 == false and .is_spamhaus == false'
```

---

## Self-Hosting

### Prerequisites

Enrichment data files must be present in `data/` before starting the server.
See [`data/README.md`](data/README.md) for full acquisition instructions.

**Quick start:**

```sh
cp data/geoipupdate.conf.example data/.geoip.conf
$EDITOR data/.geoip.conf          # add your MaxMind AccountID + LicenseKey
make -C data get_all              # download all data files
```

Without data files the service still starts and `/ip` works — geolocation,
ISP, and classification fields will return `null`.

### Docker (fastest)

```sh
docker run -p 8080:8080 \
  -v $(pwd)/data:/data \
  -e IFCONFIG_BASE_URL=localhost \
  ghcr.io/lukaspustina/ifconfig-rs:latest
```

Then visit [http://localhost:8080](http://localhost:8080) or `curl localhost:8080`.

### From Source

```sh
git clone https://github.com/lukaspustina/ifconfig-rs
cd ifconfig-rs

# 1. Build the frontend (required)
make frontend          # or: cd frontend && npm ci && npm run build

# 2. Create a config file
cp ifconfig.example.toml ifconfig.toml
$EDITOR ifconfig.toml  # set base_url and data paths

# 3. Run
cargo run -- ifconfig.toml
```

### Makefile Targets

```sh
make build        # Build frontend + cargo build
make dev          # Run dev server on :8080
make test         # Rust unit + in-process integration + frontend tests
make integration  # Docker-based integration tests only
make acceptance   # Playwright E2E tests against ip.netray.info
make bench        # Run Criterion benchmarks
make docker-build # Build production Docker image
```

---

## Configuration

Config is a TOML file (see [`ifconfig.example.toml`](ifconfig.example.toml) for all options), with `IFCONFIG_` environment variable overrides (`__` as separator).

### General

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `base_url` | string | `"localhost"` | Public domain name, used in curl examples shown in the SPA. |
| `site_name` | string | *(base_url)* | Display name shown in the site header, footer, and FAQ. Falls back to `base_url` if omitted. |
| `project_name` | string | `"ifconfig-rs"` | Project name returned by the `/meta` endpoint. |
| `project_version` | string | *(crate version)* | Project version returned by the `/meta` endpoint. |
| `filtered_headers` | string[] | `[]` | Regex patterns matched against header names. Matching headers are excluded from `/headers` responses. Useful for hiding infrastructure headers (e.g. `["^x-koyeb-", "^cf-"]`). |
| `watch_data_files` | boolean | `false` | Watch data file directories for changes and auto-reload enrichment data (like SIGHUP but filesystem-triggered). Useful for Kubernetes/Docker deployments with geoipupdate. |

### Data Files

All data file paths are optional. When omitted, the corresponding feature is disabled and affected response fields return `null` or defaults. The service starts and `/ip` works even with no data files at all.

| Key | Type | Format | Description |
|-----|------|--------|-------------|
| `geoip_city_db` | string | MaxMind MMDB | Path to GeoLite2-City database. Enables geolocation (city, country, coordinates, timezone). |
| `geoip_asn_db` | string | MaxMind MMDB | Path to GeoLite2-ASN database. Enables ISP name and ASN number lookups. Also powers the ASN name heuristic for hosting/VPN classification. |
| `user_agent_regexes` | string | YAML | Path to [ua-parser](https://github.com/ua-parser) regexes file. Enables User-Agent parsing (browser, OS, device). |
| `tor_exit_nodes` | string | Text, one IP/line | Path to Tor exit node IP list. Lines starting with `#` are ignored. Enables `is_tor` detection in the `network` object. |
| `cloud_provider_ranges` | string | JSONL | Path to normalized cloud provider CIDR file. Each line: `{"cidr":"...","provider":"...","service":"...","region":"..."}`. Enables cloud provider detection (AWS, GCP, Azure, Cloudflare, etc.). |
| `feodo_botnet_ips` | string | Text, one IP/line | Path to [Feodo Tracker](https://feodotracker.abuse.ch/) botnet C2 IP list. Enables botnet C2 detection. |
| `vpn_ranges` | string | Text, one CIDR/line | Path to VPN provider CIDR ranges (e.g. from [X4BNet](https://github.com/X4BNet/lists_vpn)). Enables CIDR-based VPN detection. ASN name heuristic still works without this. |
| `datacenter_ranges` | string | Text, one CIDR/line | Path to datacenter CIDR ranges. Enables CIDR-based datacenter detection. ASN name heuristic still works without this. |
| `bot_ranges` | string | JSONL | Path to bot CIDR file. Each line: `{"cidr":"...","provider":"..."}`. Sources: Googlebot, Bingbot, Applebot, GPTBot. Enables bot detection. |
| `spamhaus_drop` | string | Text, one CIDR/line | Path to Spamhaus DROP+EDROP+DROPv6 combined list. Enables threat/hijacked-netblock detection (`is_threat`). |
| `cins_army_ips` | string | Text, one IP/line | Path to [CINS Army](https://cinsscore.com/) bad-actor IP list. Enables `is_cins` detection in the `network` object. |

A typical `data/` directory:

```
data/
├── GeoLite2-City.mmdb              # MaxMind (free account required)
├── GeoLite2-ASN.mmdb               # MaxMind (free account required)
├── regexes.yaml                    # ua-parser/uap-core
├── tor_exit_nodes.txt              # torproject.org
├── cloud_provider_ranges.jsonl     # AWS+GCP+Azure+Cloudflare+others (via data/Makefile)
├── feodo_botnet_ips.txt            # abuse.ch
├── vpn_ranges.txt                  # X4BNet (v4+v6 merged)
├── datacenter_ranges.txt           # X4BNet datacenter list
├── bot_ranges.jsonl                # Googlebot+Bingbot+Applebot+GPTBot
├── spamhaus_drop.txt               # Spamhaus DROP+EDROP+DROPv6
└── cins_army_ips.txt               # CINS Army bad-actor list
```

See [`data/README.md`](data/README.md) for download instructions, sources, and how to keep files current.
See [`docs/enrichment.md`](docs/enrichment.md) for the full enrichment pipeline reference: confidence layers, output model, flag semantics, and per-source details.

### Server

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `server.bind` | string | `"127.0.0.1:8080"` | Address and port to bind the HTTP server. |
| `server.admin_bind` | string | *(disabled)* | Optional admin port serving Prometheus `/metrics` (application + process metrics) and `/health`. Not rate-limited — protect via network policy or `admin_token`. |
| `server.admin_token` | string | *(none)* | Bearer token required for all admin port requests. When set, every request must include `Authorization: Bearer <token>`. If unset the admin port is unauthenticated. |
| `server.trusted_proxies` | string[] | `[]` | CIDR ranges of trusted proxies for X-Forwarded-For parsing. Only the rightmost untrusted IP in the XFF chain is used as the client IP. |
| `server.cors_allowed_origins` | string[] | `["*"]` | Allowed origins for CORS. Handles OPTIONS preflight automatically. Set to specific origins to restrict cross-origin access. |

### Rate Limiting

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `rate_limit.per_ip_per_minute` | integer | `60` | Maximum sustained requests per IP per minute. |
| `rate_limit.per_ip_burst` | integer | `10` | Burst capacity — requests allowed in a quick burst before limiting kicks in. |

All rate-limited responses include `X-RateLimit-Limit` and `X-RateLimit-Remaining` headers. Responses that exceed the limit (HTTP 429) also include `Retry-After` and `X-RateLimit-Reset` (Unix timestamp) headers. `/health` and `/ready` are exempt.

### Batch

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `batch.enabled` | boolean | `false` | Enable the `POST /batch` endpoint for bulk IP lookups. |
| `batch.max_size` | integer | `100` | Maximum number of IPs per batch request. Each IP consumes one rate-limit token. |

### Cache

The `?ip=` response cache stores enrichment results in memory to reduce repeated lookups.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `cache.enabled` | boolean | `true` | Enable the in-memory LRU response cache for `?ip=` lookups. |
| `cache.ttl_secs` | integer | `300` | Time-to-live in seconds for each cached entry. |
| `cache.max_entries` | integer | `1024` | Maximum number of entries to hold in the cache. |

### Operational Features

| Feature | Description |
|---------|-------------|
| Response compression | Gzip compression via `Accept-Encoding`. Transparent — clients that don't request it get uncompressed responses. |
| Request ID | Every response includes an `X-Request-Id` header. If the client sends one, it's propagated; otherwise a unique ID is generated. Included in log spans for correlation across reverse proxies. |
| CORS | Configurable via `cors_allowed_origins`. Handles OPTIONS preflight automatically. Defaults to `["*"]`. |
| Prometheus metrics | Admin port (`/metrics`) exposes `http_requests_total`, `http_request_duration_seconds`, `enrichment_sources_loaded`, `geoip_database_age_seconds`, plus OS-level process metrics. |
| Hot-reload | SIGHUP reloads all data files without downtime. `watch_data_files = true` enables automatic filesystem-triggered reloads. |
| Structured logging | `IFCONFIG_LOG_FORMAT=json` enables JSON log output. Request ID is included in every log span. |

### Example Config

```toml
base_url = "ip.netray.info"
site_name = "My IP Service"

geoip_city_db = "data/GeoLite2-City.mmdb"
geoip_asn_db = "data/GeoLite2-ASN.mmdb"
user_agent_regexes = "data/regexes.yaml"
tor_exit_nodes = "data/tor_exit_nodes.txt"
cloud_provider_ranges = "data/cloud_provider_ranges.jsonl"
feodo_botnet_ips = "data/feodo_botnet_ips.txt"
vpn_ranges = "data/vpn_ranges.txt"
datacenter_ranges = "data/datacenter_ranges.txt"
bot_ranges = "data/bot_ranges.jsonl"
spamhaus_drop = "data/spamhaus_drop.txt"
cins_army_ips = "data/cins_army_ips.txt"

# filtered_headers = ["^x-koyeb-", "^cf-"]
# watch_data_files = true

[server]
bind = "0.0.0.0:8080"
# admin_bind = "127.0.0.1:9090"
# admin_token = "change-me"
# trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12"]
# cors_allowed_origins = ["*"]

[rate_limit]
per_ip_per_minute = 60
per_ip_burst = 10

[batch]
enabled = true
max_size = 100

[cache]
enabled = true
ttl_secs = 300
max_entries = 1024
```

### Environment Variable Overrides

Any config key can be overridden via environment variables with the `IFCONFIG_` prefix and `__` as the section separator:

```sh
IFCONFIG_BASE_URL=ip.example.com
IFCONFIG_SERVER__BIND=0.0.0.0:8080
IFCONFIG_SERVER__ADMIN_BIND=127.0.0.1:9090
IFCONFIG_SERVER__TRUSTED_PROXIES='["10.0.0.0/8"]'
IFCONFIG_RATE_LIMIT__PER_IP_PER_MINUTE=120
IFCONFIG_BATCH__ENABLED=true
IFCONFIG_BATCH__MAX_SIZE=50
IFCONFIG_SERVER__CORS_ALLOWED_ORIGINS='["https://ip.netray.info"]'
IFCONFIG_SERVER__ADMIN_TOKEN=change-me
IFCONFIG_FILTERED_HEADERS='["^x-koyeb-", "^cf-"]'
IFCONFIG_WATCH_DATA_FILES=true
IFCONFIG_LOG_FORMAT=json   # structured JSON logging (default: human-readable)
```

### CLI Flags

```sh
ifconfig-rs ifconfig.toml                  # start server with config file
ifconfig-rs --print-config ifconfig.toml   # print effective config (file + env) and exit
ifconfig-rs --check ifconfig.toml          # validate all configured data files and exit (0 = ok, 1 = error)
```

The `--check` flag is useful in deploy scripts and container health checks to verify that all data files are readable and parse correctly before starting the server.

---

## Frontend

The SolidJS SPA at `/` provides a browser interface with a few convenience features:

- **Share button** — generates a shareable `?ip=` URL using `navigator.share` when available, falling back to clipboard copy.
- **Collapsible raw JSON** — each info card (Network, Location, User Agent) has a `{·}` toggle to reveal the raw JSON for that section.
- **`?ip=` response cache** — repeated lookups for the same IP are served from an in-memory LRU cache (default: 5 min TTL, 1024 entries) to reduce latency and data-file load. Configurable via `[cache]` in the config file.

---

## FAQ

**The IP address is wrong!**

There may be proxies, load balancers, or NAT gateways between you and the server. If you're self-hosting, configure `trusted_proxies` to trust your infrastructure. If you're using [ip.netray.info](https://ip.netray.info), open a [GitHub issue](https://github.com/lukaspustina/ifconfig-rs/issues) and let's look into it.

**Does it support IPv6?**

Yes. The service is IP-version-agnostic. Use `/ipv4` or `/ipv6` if you want to force a particular version (requires your machine to have connectivity on that version).

**Can I use this in scripts?**

Absolutely — that's the primary use case. `curl` and `wget` are auto-detected and always get plain text back:

```sh
IP=$(curl -s ip.netray.info)
LOCATION=$(curl -s ip.netray.info/location)
```

Or grab structured data:

```sh
curl -s ip.netray.info/json | jq .country
curl -s ip.netray.info/all/csv
```

**Where is [ip.netray.info](https://ip.netray.info) hosted?**

On [Koyeb](https://koyeb.com).

**Can I run my own instance?**

Yes, please do! Clone this repo and follow the [Self-Hosting](#self-hosting) instructions.

---

## Postcardware

_ifconfig-rs_ is free to use and self-host. If it saves you time, I'd love a postcard from your hometown.

```
Lukas Pustina
CenterDevice GmbH
Rheinwerkallee 3
53227 Bonn
Germany
```
