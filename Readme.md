<div align="center">
  <h1>ifconfig-rs</h1>
  <p><strong>Your IP address. Instantly. From the terminal or the browser.</strong></p>
  <p>
    <a href="https://ip.pdt.sh"><img src="https://img.shields.io/badge/ip.pdt.sh-live-brightgreen.svg" alt="Live at ip.pdt.sh" /></a>
    <a href="https://github.com/lukaspustina/ifconfig-rs/actions/workflows/ci.yml"><img src="https://github.com/lukaspustina/ifconfig-rs/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
    <a href="https://github.com/lukaspustina/ifconfig-rs/releases"><img src="https://img.shields.io/github/release/lukaspustina/ifconfig-rs.svg" alt="GitHub release" /></a>
    <img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" />
  </p>
  <p>
    <a href="#quick-start">Quick Start</a> |
    <a href="#api-reference">API Reference</a> |
    <a href="#output-formats">Formats</a> |
    <a href="#self-hosting">Self-Hosting</a> |
    <a href="#configuration">Configuration</a>
  </p>
</div>

---

_ifconfig-rs_ is a fast, self-hostable "what's my IP" service written in [Rust](https://www.rust-lang.org), powered by [Axum](https://github.com/tokio-rs/axum), with a [SolidJS](https://www.solidjs.com) SPA. It serves a clean browser UI to humans and plain text to scripts — no configuration needed on the client side. Currently powering [ip.pdt.sh](https://ip.pdt.sh).

```sh
$ curl ip.pdt.sh
203.0.113.42
```

That's it. One command. Your IP. No noise.

---

## Quick Start

```sh
# Your public IP address — the most common use case
curl ip.pdt.sh

# Works great with wget and httpie too
wget -qO- ip.pdt.sh
http ip.pdt.sh

# Want more than just the IP?
curl ip.pdt.sh/all

# Everything as JSON — pipe it anywhere
curl ip.pdt.sh/json

# Just your IPv4 address (useful from dual-stack machines)
curl -4 ip.pdt.sh/ipv4

# Just your IPv6 address
curl -6 ip.pdt.sh/ipv6

# Where are you connecting from?
curl ip.pdt.sh/location

# Which ISP and ASN?
curl ip.pdt.sh/isp

# Reverse DNS hostname
curl ip.pdt.sh/host

# What headers did you send?
curl ip.pdt.sh/headers

# Health check (liveness)
curl ip.pdt.sh/health

# Readiness probe (checks GeoIP databases are loaded)
curl ip.pdt.sh/ready
```

**Pro tip:** Set up a shell alias and never think about it again:

```sh
alias myip="curl -s ip.pdt.sh"
```

---

## Why ifconfig-rs?

There are plenty of "what's my IP" services. Here's why this one is worth self-hosting:

| | ifconfig.co | wtfismyip | ipinfo.io | **ifconfig-rs** |
|---|:---:|:---:|:---:|:---:|
| Single static binary | | | | **yes** |
| No rate-limit surprises | limited | limited | limited | **configurable** |
| JSON, YAML, TOML, CSV | JSON only | JSON only | JSON only | **all four** |
| CLI auto-detection (plain text) | yes | yes | | **yes** |
| SPA with interactive API explorer | | | | **yes** |
| Dark / light / system theme | | | | **yes** |
| Tor exit node detection | | | | **yes** |
| Trusted-proxy / XFF support | | yes | | **yes** |
| Zero external runtime dependencies | | | | **yes** |
| Self-host in 5 minutes | | | | **yes** |

---

## API Reference

Every endpoint accepts a format suffix or an `Accept` header — see [Output Formats](#output-formats).

| Endpoint | Returns | Example |
|----------|---------|---------|
| `/` | Full info (SPA for browsers, plain IP for CLIs) | `curl ip.pdt.sh` |
| `/ip` | Your IP address | `curl ip.pdt.sh/ip` |
| `/ipv4` | Your IPv4 address | `curl -4 ip.pdt.sh/ipv4` |
| `/ipv6` | Your IPv6 address | `curl -6 ip.pdt.sh/ipv6` |
| `/tcp` | Your IP and source port | `curl ip.pdt.sh/tcp` |
| `/host` | Reverse DNS hostname | `curl ip.pdt.sh/host` |
| `/location` | City, region, country, coordinates | `curl ip.pdt.sh/location` |
| `/isp` | ASN number and organisation name | `curl ip.pdt.sh/isp` |
| `/network` | Network classification (type, provider, flags) | `curl ip.pdt.sh/network` |
| `/user_agent` | Parsed browser / OS / device info | `curl ip.pdt.sh/user_agent` |
| `/all` | Everything at once | `curl ip.pdt.sh/all` |
| `/headers` | Your raw request headers | `curl ip.pdt.sh/headers` |
| `/health` | Liveness probe | `curl ip.pdt.sh/health` |
| `/ready` | Readiness probe (checks GeoIP DBs) | `curl ip.pdt.sh/ready` |

### Content Negotiation

ifconfig-rs figures out what you want automatically — no flags needed:

1. **Format suffix** — `/ip/json`, `/location/yaml`, `/all/csv` — highest priority
2. **CLI detection** — `curl`, `wget`, `httpie` with `Accept: */*` get plain text
3. **`Accept` header** — standard content negotiation
4. **Default** — browsers get the SPA

---

## Output Formats

Append a format to any endpoint, or use an `Accept` header:

```sh
# Format suffix (easiest)
curl ip.pdt.sh/all/json
curl ip.pdt.sh/all/yaml
curl ip.pdt.sh/all/toml
curl ip.pdt.sh/all/csv

# Or at the root
curl ip.pdt.sh/json

# Or via Accept header
curl -H "Accept: application/json" ip.pdt.sh/all
curl -H "Accept: application/yaml" ip.pdt.sh/all
curl -H "Accept: application/toml" ip.pdt.sh/all
curl -H "Accept: text/csv"         ip.pdt.sh/all
```

| Format | Suffix | Content-Type |
|--------|--------|--------------|
| Plain text | *(CLI auto-detect)* | `text/plain` |
| JSON | `/json` | `application/json` |
| YAML | `/yaml` | `application/yaml` |
| TOML | `/toml` | `application/toml` |
| CSV | `/csv` | `text/csv` |

### Sample JSON Response

```json
{
  "ip": "203.0.113.42",
  "ip_decimal": 3405803818,
  "country": "Germany",
  "country_iso": "DE",
  "city": "Berlin",
  "hostname": "ptr-203-0-113-42.example.net",
  "latitude": 52.5200,
  "longitude": 13.4050,
  "asn": "AS1234",
  "asn_org": "Example ISP GmbH",
  "user_agent": {
    "product": "curl",
    "version": "8.6.0",
    "raw_value": "curl/8.6.0"
  }
}
```

---

## Self-Hosting

### Prerequisites

You need [MaxMind GeoLite2](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data) databases in `data/`:

```
data/GeoLite2-City.mmdb
data/GeoLite2-ASN.mmdb
```

Register for a free MaxMind account to download them. Without the databases, geolocation and ISP endpoints return empty results (the service still starts and `/ip` still works).

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
make tests        # Unit + Docker integration + Playwright E2E
make integration  # Docker-based integration tests only
make acceptance   # Playwright E2E tests against ip.pdt.sh
make docker-build # Build production Docker image
```

---

## Configuration

Config is a TOML file (see [`ifconfig.example.toml`](ifconfig.example.toml) for all options), with `IFCONFIG_` environment variable overrides (`__` as separator).

```toml
# Public domain, used in CLI examples shown in the SPA
base_url = "ip.pdt.sh"

# Display name in the site header and footer (defaults to base_url)
site_name = "My IP Service"

# MaxMind GeoLite2 databases
geoip_city_db  = "data/GeoLite2-City.mmdb"
geoip_asn_db   = "data/GeoLite2-ASN.mmdb"

# ua-parser regexes for User-Agent parsing
user_agent_regexes = "data/regexes.yaml"

# Newline-delimited list of Tor exit node IPs
tor_exit_nodes = "data/tor_exit_nodes.txt"

[server]
bind = "0.0.0.0:8080"

# Optional admin port for Prometheus /metrics and /health (disabled by default)
# admin_bind = "127.0.0.1:9090"

# Trust these CIDR ranges for X-Forwarded-For (e.g. behind a load balancer)
# trusted_proxies = ["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16"]

[rate_limit]
per_ip_per_minute = 60   # sustained request rate
per_ip_burst      = 10   # burst capacity
```

**Environment variable examples:**

```sh
IFCONFIG_BASE_URL=ip.example.com
IFCONFIG_SERVER__BIND=0.0.0.0:9090
IFCONFIG_SERVER__ADMIN_BIND=127.0.0.1:9090
IFCONFIG_RATE_LIMIT__PER_IP_PER_MINUTE=120
IFCONFIG_LOG_FORMAT=json   # structured JSON logging (default: human-readable)
```

### Behind a Load Balancer

Set `trusted_proxies` to the CIDR ranges of your load balancers so that `X-Forwarded-For` is trusted and the real client IP is shown:

```toml
[server]
trusted_proxies = ["10.0.0.0/8"]
```

Without this, the IP of the load balancer would be reported as the client IP.

### CLI Flags

```sh
cargo run -- ifconfig.toml              # start server with config file
cargo run -- --print-config ifconfig.toml  # print effective config (file + env) and exit
```

### Admin Port / Prometheus Metrics

Set `server.admin_bind` to expose a separate admin listener with `/metrics` (Prometheus exposition format) and `/health`:

```toml
[server]
admin_bind = "127.0.0.1:9090"
```

The `/metrics` endpoint includes process-level metrics (CPU, memory, file descriptors). Disabled by default.

### Rate Limit Headers

All rate-limited responses include `X-RateLimit-Limit` and `X-RateLimit-Remaining` headers. Responses that exceed the limit (HTTP 429) also include a `Retry-After` header. The `/health` and `/ready` endpoints are exempt from rate limiting.

---

## FAQ

**The IP address is wrong!**

There may be proxies, load balancers, or NAT gateways between you and the server. If you're self-hosting, configure `trusted_proxies` to trust your infrastructure. If you're using [ip.pdt.sh](https://ip.pdt.sh), open a [GitHub issue](https://github.com/lukaspustina/ifconfig-rs/issues) and let's look into it.

**Does it support IPv6?**

Yes. The service is IP-version-agnostic. Use `/ipv4` or `/ipv6` if you want to force a particular version (requires your machine to have connectivity on that version).

**Can I use this in scripts?**

Absolutely — that's the primary use case. `curl` and `wget` are auto-detected and always get plain text back:

```sh
IP=$(curl -s ip.pdt.sh)
LOCATION=$(curl -s ip.pdt.sh/location)
```

Or grab structured data:

```sh
curl -s ip.pdt.sh/json | jq .country
curl -s ip.pdt.sh/all/csv
```

**Where is [ip.pdt.sh](https://ip.pdt.sh) hosted?**

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
