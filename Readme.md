# ifconfig-rs

[![Production deployed](https://img.shields.io/badge/ip.pdt.sh-prod-brightgreen.svg)](https://ip.pdt.sh) [![Build Status](https://github.com/lukaspustina/ifconfig-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/lukaspustina/ifconfig-rs/actions/workflows/ci.yml) [![GitHub release](https://img.shields.io/github/release/lukaspustina/ifconfig-rs.svg)](https://github.com/lukaspustina/ifconfig-rs/releases) [![license](https://img.shields.io/github/license/lukaspustina/ifconfig-rs.svg)](https://github.com/lukaspustina/ifconfig-rs/blob/master/LICENSE)

_ifconfig-rs_ is a fast, self-hostable <a href="https://www.google.com/search?q=what's+my+ip+address">"what's my IP address"</a> service currently powering [ip.pdt.sh](https://ip.pdt.sh). Written in [Rust](https://www.rust-lang.org) using the [Axum](https://github.com/tokio-rs/axum) web framework, with a [SolidJS](https://www.solidjs.com) SPA frontend. Includes GeoLite2 data created by MaxMind, available from [http://www.maxmind.com](http://www.maxmind.com). MIT licensed.

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->
**Table of Contents**

- [Features](#features)
- [Another "What's my IP" service? But why?](#another-whats-my-ip-service-but-why)
- [Customization](#customization)
- [Deployment Prerequisites](#deployment-prerequisites)
- [Koyeb (and other providers using load balancers)](#koyeb-and-other-providers-using-load-balancers)
- [FAQ](#faq)
- [Postcardware](#postcardware)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Features

* Fast — single Rust binary with embedded frontend assets.

* Shows your IP address, TCP port, host name, geoip-based location, ISP, and user agent.

* Google Maps integration for geoip location.

* Multi-format API: JSON, YAML, TOML, CSV, and plain text.

* Special [support for CLI tools](https://ip.pdt.sh) like [curl](https://curl.haxx.se), [httpie](https://github.com/jakubroztocil/httpie), and [wget](https://www.gnu.org/software/wget/) — API calls return just the value followed by a newline for easy script integration.

* Interactive SPA with API Explorer and dark/light/system theme support.

* Per-IP rate limiting with configurable burst and per-minute caps.

* Tor exit node detection.


## Another "What's my IP" service? But why?

First of all, everybody should have a "What's my IP" service. I wanted a small web project in Rust and was strongly inspired by [ipd](https://github.com/mpolden/ipd) which powers [ifconfig.co](https://ifconfig.co). ifconfig-rs adds a few details and has a nicer UI.


## Customization

Runtime configuration is loaded from a TOML file (see `ifconfig.example.toml` for all options) with `IFCONFIG_` environment variable overrides.

```toml
base_url = "localhost"
site_name = "ifconfig-rs"
project_name = "ifconfig-rs"
geoip_city_db = "data/GeoLite2-City.mmdb"
geoip_asn_db = "data/GeoLite2-ASN.mmdb"
user_agent_regexes = "data/regexes.yaml"
tor_exit_nodes = "data/tor_exit_nodes.txt"

[server]
bind = "127.0.0.1:8080"
# trusted_proxies = ["10.0.0.0/8"]

[rate_limit]
per_ip_per_minute = 60
per_ip_burst = 10
```

Environment variable examples: `IFCONFIG_SERVER__BIND=0.0.0.0:8080`, `IFCONFIG_BASE_URL=ip.pdt.sh`.

### `base_url`

Sets the base URL used in CLI examples shown in the SPA and plain-text responses.

### `site_name` / `project_name`

Sets the title and name displayed in the UI.

### `trusted_proxies`

List of CIDR ranges for trusted reverse proxies. When set, `X-Forwarded-For` is consulted only from these sources to determine the client IP.

### Rate limiting

`per_ip_per_minute` and `per_ip_burst` control the token-bucket rate limiter applied per client IP.


## Deployment Prerequisites

You need GeoIP databases from MaxMind in `data/`:

* `data/GeoLite2-City.mmdb`
* `data/GeoLite2-ASN.mmdb`

Register for a free MaxMind account at [https://www.maxmind.com](https://www.maxmind.com) to download these files.

Build the frontend before `cargo build`:

```sh
cd frontend && npm ci && npm run build
cargo build
```

Or use the Makefile:

```sh
make build   # builds frontend then cargo build
make dev     # runs the dev server on :8080
```

### Docker

```sh
make docker-build
docker run -p 8080:8080 -v $(pwd)/data:/data ifconfig-rs:0.4.0 /ifconfig.toml
```


## Koyeb (and other providers using load balancers)

[Koyeb](https://koyeb.com) and similar platforms use load balancers that masquerade the original client IP. Configure `trusted_proxies` in the TOML config to trust the load balancer CIDR range, so that `X-Forwarded-For` is used to extract the real client IP.


## FAQ

* **The IP address is wrong!**

  This can happen due to transparent proxies, load balancers, or NAT gateways between the client and the server. Configure `trusted_proxies` appropriately. Open an issue on [GitHub](https://github.com/lukaspustina/ifconfig-rs/issues) if you encounter a case the heuristic should handle.

* **Where is [ip.pdt.sh](https://ip.pdt.sh) hosted?**

  The code runs on [Koyeb](https://koyeb.com).

* **Does _ifconfig-rs_ support IPv6?**

  Yes. The code is IP-version-agnostic.

* **Can I run my own instance?**

  Yes. Clone or fork this repository. If you find it useful, a postcard would be appreciated — see below.

* **Can you add &lt;feature&gt;?**

  Open an issue or send a pull request.


## Postcardware

You're free to use _ifconfig-rs_. If you find it useful, I would highly appreciate you sending me a postcard from your hometown. My work address is

```
Lukas Pustina
CenterDevice GmbH
Rheinwerkallee 3
53227 Bonn
Germany
```
