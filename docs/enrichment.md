# Enrichment Pipeline

ifconfig-rs enriches each IP address with data from multiple sources loaded at startup
(and optionally hot-reloaded via SIGHUP or filesystem watch). This document describes
every data source, how confidence is layered, and exactly how the output fields are
derived.

---

## Data Sources

| Config key | File | Source | Update cadence | What it provides |
|---|---|---|---|---|
| `geoip_city_db` | `GeoLite2-City.mmdb` | [MaxMind GeoLite2](https://www.maxmind.com/en/geolite2/signup) | 1st + 3rd Tuesday/month | City, region, country, coordinates, timezone, `is_eu`, accuracy radius |
| `geoip_asn_db` | `GeoLite2-ASN.mmdb` | [MaxMind GeoLite2](https://www.maxmind.com/en/geolite2/signup) | 1st + 3rd Tuesday/month | ASN number, org name, IP prefix |
| `user_agent_regexes` | `regexes.yaml` | [ua-parser/uap-core](https://github.com/ua-parser/uap-core/blob/master/regexes.yaml) | On release | Browser, OS, and device family/version parsing |
| `cloud_provider_ranges` | `cloud_provider_ranges.jsonl` | AWS, GCP, Azure, Cloudflare, Oracle, Fastly, DigitalOcean, Linode, GitHub, Google Services (normalized to JSONL) | Weekly | Cloud provider identity: `provider`, `service`, `region` |
| `bot_ranges` | `bot_ranges.jsonl` | Googlebot, Bingbot, Applebot, GPTBot (official publisher lists, normalized to JSONL) | Weekly | Bot crawler identity: `provider` |
| `feodo_botnet_ips` | `feodo_botnet_ips.txt` | [Feodo Tracker / abuse.ch](https://feodotracker.abuse.ch/downloads/ipblocklist.txt) | Daily | Active botnet C2 nodes (`is_c2`) |
| `spamhaus_drop` | `spamhaus_drop.txt` | [Spamhaus DROP + EDROP + DROPv6](https://www.spamhaus.org/drop/) | Daily | Hijacked netblocks (`is_spamhaus`) |
| `tor_exit_nodes` | `tor_exit_nodes.txt` | [Tor Project bulk exit list](https://check.torproject.org/torbulkexitlist) | Hourly | Tor exit nodes (`is_tor`) |
| `vpn_ranges` | `vpn_ranges.txt` | [X4BNet lists_vpn](https://github.com/X4BNet/lists_vpn) (IPv4 + IPv6) | Weekly | VPN CIDR ranges (`is_vpn`) |
| `datacenter_ranges` | `datacenter_ranges.txt` | [X4BNet datacenter list](https://github.com/X4BNet/lists_vpn) | Weekly | Generic datacenter CIDRs (`is_datacenter`) |
| `asn_patterns` | TOML file (optional) | Built-in defaults or external file | On reload | ASN org-name patterns for hosting/VPN providers without official CIDR lists |
| `asn_info` | `as_metadata.jsonl` | [ipverse/as-metadata](https://github.com/ipverse/as-metadata) | Weekly | ASN category (`asn_category`) and network role (`network_role`) |

### Source availability

**Mandatory** (process exits if configured but fails to load): `geoip_city_db`,
`geoip_asn_db`, `user_agent_regexes`.

**Optional** (warning logged, detection disabled): all other sources. When optional
sources are configured but fail to load they appear in the `/ready` endpoint `warnings`
array and are tracked by the `enrichment_sources_loaded{source}` Prometheus gauge.

---

## Four-Layer Confidence Model

Enrichment signals are produced by four layers in increasing order of uncertainty.
Higher-layer signals are not discarded but may be overridden in `type` priority ordering.

### Layer 0 — Deterministic IP Math

Evaluated before any file lookup. RFC 1918 (`10/8`, `172.16/12`, `192.168/16`),
loopback (`127/8`, `::1`), link-local (`169.254/16`, `fe80::/10`), and ULA (`fc00::/7`)
are unconditionally classified as `is_internal = true`. No data file is consulted.

### Layer 1 — Authoritative Vendor / Threat Intel CIDRs

CIDR lists published directly by the operator of the resource:

- **Cloud provider ranges** — IP ranges published by AWS, GCP, Azure, Cloudflare, and
  other vendors in their official JSON feeds. Match gives `cloud: { provider, service?,
  region? }` with the exact service and region from the vendor metadata.
- **Bot ranges** — IP ranges published by Googlebot, Bingbot, Applebot, and GPTBot in
  their respective crawl verification documentation. Match gives `bot: { provider }`.
- **Feodo Tracker** — Active C2 botnet infrastructure IPs maintained by abuse.ch.
  Match gives `is_c2 = true`. These are known command-and-control nodes with high
  confidence; the list is manually curated.
- **Spamhaus DROP/EDROP/DROPv6** — Netblocks that Spamhaus considers entirely hijacked
  or under the control of criminal organizations. Match gives `is_spamhaus = true`.
  These are netblocks, not individual IPs — an entire prefix is considered hostile.

### Layer 2 — Community CIDR Lists

Crowdsourced or derived lists with good but not authoritative coverage:

- **VPN ranges** (X4BNet) — CIDR aggregates of known commercial VPN exit nodes. Match
  gives `is_vpn = true`. No provider identity from this source alone — provider name
  may be added by Layer 3 (ASN heuristic).
- **Datacenter ranges** (X4BNet) — Generic hosting/datacenter CIDRs not covered by
  the cloud vendor lists. Match gives `is_datacenter = true`.
- **Tor exit nodes** (Tor Project) — Authoritative list of current Tor exit relays from
  the Tor Project's bulk exit list endpoint. Match gives `is_tor = true`. Treated as
  community-tier because the list changes frequently and a stale snapshot degrades
  confidence.

### Layer 3 — ASN-Keyed Data

Data keyed by Autonomous System Number rather than individual prefix:

- **ASN heuristic** (`asn_heuristic.rs`) — Case-insensitive substring match or exact
  ASN number match against the MaxMind org name. Covers ~33 hosting providers and ~12
  VPN providers that do not publish machine-readable CIDR lists. When the VPN heuristic
  matches, it adds `vpn: { provider }` to an existing CIDR match or produces
  `is_vpn = true` alone. Patterns are loaded from `asn_patterns` (TOML) at startup;
  falls back to compiled-in defaults when unconfigured.
- **ipverse as-metadata** (`asn_info`) — Per-ASN category and network role derived from
  RPKI/IRR routing data. Provides `asn_category` and `network_role`.

### Layer 4 — GeoIP and Async Derived

- **MaxMind GeoLite2-City** — City, region, country, coordinates, timezone, `is_eu`,
  accuracy radius. Loaded from a MaxMind binary database via `maxminddb`.
- **MaxMind GeoLite2-ASN** — ASN number, org name, and announced prefix. Used as
  input to Layer 3 lookups.
- **PTR reverse DNS** — Async DNS query for the PTR record of the IP. Performed by
  default for `?ip=` lookups; skipped by default in batch for performance (opt in via
  `?dns=true`).

---

## Two-Dimension Output Model

The `Network` object exposes two complementary dimensions to avoid collapsing orthogonal
signals into a single value.

### `type` — Priority-Ordered Most Notable Signal

A single string summarising the most security- or operationally-relevant characteristic
of the IP. Priority order (highest wins):

| Priority | Value | Condition |
|---|---|---|
| 1 | `"internal"` | `is_internal = true` |
| 2 | `"c2"` | `is_c2 = true` |
| 3 | `"bot"` | `is_bot = true` |
| 4 | `"cloud"` | `cloud != null` |
| 5 | `"vpn"` | `is_vpn = true` |
| 6 | `"tor"` | `is_tor = true` |
| 7 | `"spamhaus"` | `is_spamhaus = true` |
| 8 | `"datacenter"` | `is_datacenter = true` |
| 9 | `"residential"` | (default / catch-all) |

Cloud is ranked above VPN because cloud CIDR matches come from authoritative vendor
feeds; VPN CIDR lists have community-tier confidence and some cloud-hosted VPN exits
could match both. C2 and bot are ranked highest among non-internal signals because
they indicate active malicious or crawler infrastructure.

### `infra_type` — Infrastructure Dimension

An orthogonal dimension describing what kind of network infrastructure the IP belongs to,
independent of its threat/service role:

| Value | Condition |
|---|---|
| `"internal"` | `is_internal = true` |
| `"cloud"` | `cloud != null` |
| `"datacenter"` | `is_datacenter = true` (and not cloud) |
| `"government"` | `asn_category == "government_admin"` |
| `"education"` | `asn_category == "education_research"` |
| `"business"` | `asn_category == "business"` |
| `"residential"` | (default — `isp`, unknown, or unmatched) |

An AWS IP that is also a known Feodo C2 node will have `type = "c2"` and
`infra_type = "cloud"` — both dimensions are preserved.

---

## Typed Identity Objects

The three identity objects provide richer detail than boolean flags alone.

### `cloud: CloudInfo | null`

Populated when `cloud_provider_ranges` CIDR match succeeds.

```json
{ "provider": "aws", "service": "EC2", "region": "us-east-1" }
```

`provider` is always present. `service` and `region` are included only when the vendor's
published feed includes them (AWS and Azure include region; GCP includes service name).
The `provider` slug is normalised to lowercase: `aws`, `gcp`, `azure`, `cloudflare`,
`digitalocean`, `linode`, `hetzner`, `ovh`, `oracle`, `ibm`, `vultr`, `google-services`.

### `vpn: VpnInfo | null`

Populated when `is_vpn = true` (from CIDR match or ASN heuristic).

```json
{ "provider": "Mullvad" }
```

`provider` is `null` when only the CIDR list matched (X4BNet does not include provider
names). `provider` is set when the ASN heuristic also matched, naming the VPN service.

### `bot: NetworkBot | null`

Populated when `bot_ranges` CIDR match succeeds.

```json
{ "provider": "googlebot" }
```

`provider` is the crawler identifier from the JSONL bot ranges file (e.g. `"googlebot"`,
`"bingbot"`, `"applebot"`, `"gptbot"`).

---

## ASN Category and Network Role

Both fields come from `asn_info` (ipverse/as-metadata) and are keyed by ASN number.

### `asn_category`

Broad industry category of the ASN operator:

| Value | Meaning |
|---|---|
| `"hosting"` | Cloud/hosting provider |
| `"isp"` | Internet service provider (residential/broadband) |
| `"business"` | Enterprise/corporate network |
| `"education_research"` | University or research institution |
| `"government_admin"` | Government or public administration |
| `null` | Unknown or ASN not in dataset |

Used in `infra_type` derivation for government, education, and business classifications.

### `network_role`

Routing role of the ASN in the global BGP topology:

| Value | Meaning |
|---|---|
| `"tier1_transit"` | Major transit provider (full mesh, no upstreams) |
| `"stub"` | Stub network (single upstream, no transit) |
| `"access_provider"` | Last-mile access network |
| `"content_network"` | CDN or content delivery network |
| `"ix_route_server"` | Internet exchange route server |
| `null` | Unknown or not in dataset |

Displayed as a badge in the UI and shown in the Network card's Role row.

---

## Flag Reference

| Field | Source | Meaning for operators |
|---|---|---|
| `is_internal` | IP math (Layer 0) | RFC 1918 / loopback / ULA — no external routing |
| `is_c2` | Feodo Tracker (Layer 1) | Confirmed active botnet C2 infrastructure — block immediately |
| `is_bot` | Bot CIDR ranges (Layer 1) | Verified crawler — usually safe, but may inflate analytics |
| `is_spamhaus` | Spamhaus DROP/EDROP (Layer 1) | Hijacked netblock — high abuse probability across the entire prefix |
| `is_tor` | Tor exit list (Layer 2) | Tor exit relay — anonymized origin, no real geolocation possible |
| `is_vpn` | VPN CIDR ranges (Layer 2) + ASN heuristic (Layer 3) | Commercial VPN — real location and identity hidden |
| `is_datacenter` | Datacenter CIDR ranges (Layer 2) | Generic hosting, not a named cloud provider |

### `is_c2` vs `is_spamhaus`

These are distinct threat categories and should be treated differently:

- **`is_c2`** (Feodo Tracker): The IP is an active command-and-control node for a botnet
  (typically Emotet, QakBot, or similar). The IP itself is malicious infrastructure —
  block inbound connections from it and flag any outbound connections to it.
- **`is_spamhaus`** (Spamhaus DROP/EDROP): The entire netblock is listed as hijacked or
  under criminal control. Traffic may come from any IP in the prefix. Use for network
  ingress filtering; the whole subnet is suspect, not just this one IP.
