# Data Sources

ifconfig-rs reads all enrichment data from local files at startup.
This document explains where to obtain each file and how to keep it up to date.

Run `make update-data` to refresh the files that have public download URLs.
Files that require a MaxMind account are handled by `geoipupdate` (see below).

---

## MaxMind GeoLite2 (GeoIP City + ASN)

**Files:** `data/GeoLite2-City.mmdb`, `data/GeoLite2-ASN.mmdb`

MaxMind distributes these databases under the [GeoLite2 End User License Agreement](https://www.maxmind.com/en/geolite2/eula).
A free MaxMind account is required.

1. [Create a MaxMind account](https://www.maxmind.com/en/geolite2/signup) and generate a license key in *My Account → Services → Manage License Keys*.
2. Install [`geoipupdate`](https://github.com/maxmind/geoipupdate):
   - macOS: `brew install geoipupdate`
   - Debian/Ubuntu: follow the [official guide](https://github.com/maxmind/geoipupdate#installing-on-ubuntu-via-apt)
   - Docker: `ghcr.io/maxmind/geoipupdate`
3. Place your config at `~/.config/GeoIP.conf` (or `/etc/GeoIP.conf`) — see `docs/geoipupdate.conf.example`.
4. Run `geoipupdate` (or `make update-data`) to download/refresh the `.mmdb` files into `data/`.

MaxMind releases updated databases on the first and third Tuesday of each month.
In production, run `geoipupdate` on a weekly cron and send SIGHUP to the server,
or set `watch_data_files = true` in the config.

---

## User-Agent Regexes

**File:** `data/regexes.yaml`

Source: [ua-parser/uap-core](https://github.com/ua-parser/uap-core/blob/master/regexes.yaml)

```sh
curl -fsSL https://raw.githubusercontent.com/ua-parser/uap-core/master/regexes.yaml \
     -o data/regexes.yaml
```

---

## Tor Exit Nodes

**File:** `data/tor_exit_nodes.txt`

Source: [Tor Project bulk exit list](https://check.torproject.org/torbulkexitlist) — one IP per line, updated in near-real-time.

```sh
curl -fsSL https://check.torproject.org/torbulkexitlist -o data/tor_exit_nodes.txt
```

---

## Feodo Botnet C2 IPs

**File:** `data/feodo_botnet_ips.txt`

Source: [Feodo Tracker recommended IP blocklist](https://feodotracker.abuse.ch/downloads/ipblocklist_recommended.txt) — IPs of active botnet C2 servers, maintained by abuse.ch.

```sh
curl -fsSL https://feodotracker.abuse.ch/downloads/ipblocklist_recommended.txt \
     -o data/feodo_botnet_ips.txt
```

---

## Spamhaus DROP / EDROP / DROPv6

**File:** `data/spamhaus_drop.txt`

Source: [Spamhaus Don't Route Or Peer lists](https://www.spamhaus.org/drop/) — CIDR blocks operated by spam/malware networks.
The file should contain DROP + EDROP + DROPv6 concatenated.

```sh
{ curl -fsSL https://www.spamhaus.org/drop/drop.txt
  curl -fsSL https://www.spamhaus.org/drop/edrop.txt
  curl -fsSL https://www.spamhaus.org/drop/dropv6.txt
} > data/spamhaus_drop.txt
```

> Note: Spamhaus requires a [data feed license](https://www.spamhaus.com/pricing/) for commercial use.
> The free feeds are rate-limited and intended for personal/non-commercial use.

---

## Cloud Provider CIDR Ranges

**File:** `data/cloud_provider_ranges.jsonl`

Format: one JSON object per line — `{"cidr":"...","provider":"...","service":"...","region":"..."}`.

Maintained manually from public IP range publications:

| Provider | Source |
|---|---|
| AWS | <https://ip-ranges.amazonaws.com/ip-ranges.json> |
| GCP | <https://www.gstatic.com/ipranges/cloud.json> |
| Azure | <https://www.microsoft.com/en-us/download/details.aspx?id=56519> |
| Cloudflare | <https://www.cloudflare.com/ips/> |
| Fastly | <https://api.fastly.com/public-ip-list> |
| Oracle | <https://docs.oracle.com/en-us/iaas/tools/public_ip_ranges.json> |
| DigitalOcean | <https://www.digitalocean.com/geo/google.csv> |

The ifconfig-rs project maintains a pre-built JSONL in the `ifconfig-rs-data` Docker image
(`ghcr.io/lukaspustina/ifconfig-rs-data:latest`), updated periodically.

---

## VPN CIDR Ranges

**File:** `data/vpn_ranges.txt`

Format: one CIDR per line. Aggregated from various public VPN provider IP publications.
No standard public download; maintained manually in the `ifconfig-rs-data` image.

---

## Datacenter CIDR Ranges

**File:** `data/datacenter_ranges.txt`

Format: one CIDR per line.

Source: [X4BNet datacenter ranges](https://github.com/X4BNet/lists_vpn) and similar aggregations.

```sh
curl -fsSL https://raw.githubusercontent.com/X4BNet/lists_vpn/main/output/datacenter/ipv4.txt \
     -o data/datacenter_ranges.txt
```

---

## Bot CIDR Ranges

**File:** `data/bot_ranges.jsonl`

Format: one JSON object per line — `{"cidr":"...","provider":"..."}`.

Sources: IP ranges published by major bot providers (Googlebot, Bingbot, etc.).
Maintained in the `ifconfig-rs-data` image.

---

## Using the `ifconfig-rs-data` Docker Image

The easiest way to populate all data files at once is to copy them from the pre-built image:

```sh
docker create --name ifconfig-data ghcr.io/lukaspustina/ifconfig-rs-data:latest
docker cp ifconfig-data:/data ./data
docker rm ifconfig-data
```

---

## Keeping Data Up to Date

### Production (recommended)

Set `watch_data_files = true` in your config and run `geoipupdate` on a weekly cron.
The server reloads enrichment data automatically when files change.

### Manual reload

Send SIGHUP to the running process:

```sh
kill -HUP $(pidof ifconfig-rs)
```

Or, with Docker:

```sh
docker kill --signal=HUP <container>
```
