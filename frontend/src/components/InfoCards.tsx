import { Show } from "solid-js";
import type { Ifconfig } from "../lib/types";

interface Props {
  data: Ifconfig;
}

/** Treat null, undefined, and "unknown" as missing. */
function known(v: string | null | undefined): string | null {
  return v && v !== "unknown" ? v : null;
}

/** Convert GeoIP accuracy radius (km) to a Google Maps zoom level. */
function radiusToZoom(radiusKm: number): number {
  const zoom = Math.log2(40075 / (radiusKm * 4));
  return Math.round(Math.min(Math.max(zoom, 3), 15));
}

/** Title-case a string (replace underscores with spaces, capitalize each word). */
function titleCase(s: string): string {
  return s.replace(/_/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

/** Display name for a cloud provider slug. Falls back to capitalizing words. */
function cloudDisplay(provider: string): string {
  const names: Record<string, string> = {
    "aws": "AWS",
    "gcp": "GCP",
    "azure": "Azure",
    "cloudflare": "Cloudflare",
    "digitalocean": "DigitalOcean",
    "linode": "Linode",
    "hetzner": "Hetzner",
    "ovh": "OVH",
    "oracle": "Oracle",
    "ibm": "IBM",
    "vultr": "Vultr",
    "google-services": "Google",
  };
  return names[provider] ?? provider.replace(/[-_]/g, " ").replace(/\b\w/g, (c) => c.toUpperCase());
}

export default function InfoCards(props: Props) {
  const loc = () => props.data.location;
  const net = () => props.data.network;

  const mapsUrl = () => {
    const { latitude, longitude, accuracy_radius_km } = loc();
    if (latitude == null || longitude == null) return null;
    const zoom = accuracy_radius_km != null ? radiusToZoom(accuracy_radius_km) : 10;
    return `https://www.google.com/maps/@${latitude},${longitude},${zoom}z`;
  };

  return (
    <>
    <div class="badge-bar">
      <span class="net-badge net-badge--version">IPv{props.data.ip.version}</span>
      <Show when={loc().is_eu === true}>
        <span class="net-badge net-badge--eu">EU</span>
      </Show>
      <Show when={loc().is_eu === false}>
        <span class="net-badge net-badge--non-eu">non-EU</span>
      </Show>
      {/* infra_type badge — hidden for "residential" (the default, absence is implicit) */}
      <Show when={net().infra_type !== "residential"}>
        <span class={`net-badge net-badge--${net().infra_type}`}>{net().infra_type}</span>
      </Show>
      {/* Cloud badge — shown when cloud identity is known */}
      <Show when={net().cloud != null}>
        <span class="net-badge net-badge--cloud">
          {cloudDisplay(net().cloud!.provider)}
          {net().cloud!.service ? ` · ${net().cloud!.service}` : ""}
        </span>
      </Show>
      {/* VPN badge */}
      <Show when={net().is_vpn}>
        <span class="net-badge net-badge--vpn">
          {net().vpn!.provider ?? "VPN"}
        </span>
      </Show>
      {/* Tor badge */}
      <Show when={net().is_tor}>
        <span class="net-badge net-badge--tor">Tor</span>
      </Show>
      {/* Bot badge */}
      <Show when={net().is_bot}>
        <span class="net-badge net-badge--bot">
          {titleCase(net().bot!.provider)}
        </span>
      </Show>
      {/* C2 badge — active Feodo botnet C2 node */}
      <Show when={net().is_c2}>
        <span class="net-badge net-badge--c2">C2</span>
      </Show>
      {/* Spamhaus badge — hijacked netblock */}
      <Show when={net().is_spamhaus}>
        <span class="net-badge net-badge--spamhaus">Spamhaus</span>
      </Show>
      {/* Network role badge */}
      <Show when={net().network_role != null}>
        <span class="net-badge net-badge--role">
          {net().network_role!.replace(/_/g, " ")}
        </span>
      </Show>
    </div>
    <div class="cards">
      {/* Network Card */}
      <div class="card">
        <div class="card-title">Network</div>
        <div class="card-row">
          <span class="card-label">IP Address</span>
          <span class="card-value mono">{props.data.ip.addr}</span>
        </div>
        <div class="card-row">
          <span class="card-label">IP Version</span>
          <span class="card-value">IPv{props.data.ip.version}</span>
        </div>
        <Show when={props.data.tcp}>
          <div class="card-row">
            <span class="card-label">TCP Port</span>
            <span class="card-value">{props.data.tcp!.port}</span>
          </div>
        </Show>
        <Show when={props.data.ip.hostname}>
          <div class="card-row card-row-stackable">
            <span class="card-label">Hostname</span>
            <span class="card-value">{props.data.ip.hostname}</span>
          </div>
        </Show>
        <Show when={net().asn != null || net().prefix != null}>
          <div class="card-row">
            <span class="card-label">ASN</span>
            <span class="card-value mono">
              {[net().asn != null ? `AS${net().asn}` : null, net().prefix].filter(Boolean).join(" · ")}
            </span>
          </div>
        </Show>
        <Show when={net().org != null}>
          <div class="card-row card-row-stackable">
            <span class="card-label">Org</span>
            <span class="card-value">{net().org}</span>
          </div>
        </Show>
        <Show when={net().asn_category != null}>
          <div class="card-row">
            <span class="card-label">Category</span>
            <span class="card-value">{titleCase(net().asn_category!)}</span>
          </div>
        </Show>
        <Show when={net().cloud != null}>
          <div class="card-row card-row-stackable">
            <span class="card-label">Cloud</span>
            <span class="card-value">
              {[
                net().cloud!.provider,
                net().cloud!.service,
                net().cloud!.region,
              ].filter(Boolean).join(" · ")}
            </span>
          </div>
        </Show>
        <Show when={net().vpn != null}>
          <div class="card-row">
            <span class="card-label">VPN</span>
            <span class="card-value">{net().vpn!.provider ?? "—"}</span>
          </div>
        </Show>
        <Show when={net().bot != null}>
          <div class="card-row">
            <span class="card-label">Bot</span>
            <span class="card-value">{titleCase(net().bot!.provider)}</span>
          </div>
        </Show>
        <Show when={net().network_role != null}>
          <div class="card-row">
            <span class="card-label">Role</span>
            <span class="card-value">{titleCase(net().network_role!)}</span>
          </div>
        </Show>
      </div>

      {/* User Agent Card */}
      <Show when={props.data.user_agent}>
        <div class="card">
          <div class="card-title">User Agent</div>
          <div class="card-row">
            <span class="card-label">Browser</span>
            <span class="card-value">
              {props.data.user_agent!.browser.family}{" "}
              {props.data.user_agent!.browser.version}
            </span>
          </div>
          <div class="card-row">
            <span class="card-label">OS</span>
            <span class="card-value">
              {props.data.user_agent!.os.family}{" "}
              {props.data.user_agent!.os.version}
            </span>
          </div>
          <Show when={props.data.user_agent!.device.family !== "Other"}>
            <div class="card-row">
              <span class="card-label">Device</span>
              <span class="card-value">
                {[
                  props.data.user_agent!.device.brand,
                  props.data.user_agent!.device.model,
                  props.data.user_agent!.device.family,
                ]
                  .filter(Boolean)
                  .join(" ")}
              </span>
            </div>
          </Show>
        </div>
      </Show>

      {/* Location Card */}
      <div class="card">
        <div class="card-title">Location</div>
        <Show when={known(loc().city)}>
          <div class="card-row">
            <span class="card-label">City</span>
            <span class="card-value">
              {mapsUrl() ? (
                <a class="map-link" href={mapsUrl()!} target="_blank" rel="noopener noreferrer" title="Open in Google Maps">
                  {loc().city}
                </a>
              ) : (
                loc().city
              )}
            </span>
          </div>
        </Show>
        <Show when={loc().latitude != null && loc().longitude != null}>
          <div class="card-row">
            <span class="card-label">Coordinates</span>
            <span class="card-value">
              {mapsUrl() ? (
                <a class="map-link" href={mapsUrl()!} target="_blank" rel="noopener noreferrer" title="Open in Google Maps">
                  {loc().latitude!.toFixed(4)}, {loc().longitude!.toFixed(4)}
                </a>
              ) : (
                <>{loc().latitude!.toFixed(4)}, {loc().longitude!.toFixed(4)}</>
              )}
              <Show when={loc().accuracy_radius_km != null}>
                <span class="accuracy-hint" title="GeoIP accuracy radius">
                  {" "}±{loc().accuracy_radius_km}km
                </span>
              </Show>
            </span>
          </div>
        </Show>
        <Show when={known(loc().region)}>
          <div class="card-row">
            <span class="card-label">Region</span>
            <span class="card-value">
              {loc().region}
              <Show when={known(loc().region_code)}>
                {" "}({loc().region_code})
              </Show>
            </span>
          </div>
        </Show>
        <Show when={known(loc().country)}>
          <div class="card-row">
            <span class="card-label">Country</span>
            <span class="card-value">
              {loc().country}
              <Show when={known(loc().country_iso)}>
                {" "}({loc().country_iso})
              </Show>
              <Show when={known(loc().continent)}>
                {" · "}{loc().continent}
              </Show>
            </span>
          </div>
        </Show>
        <Show when={known(loc().timezone)}>
          <div class="card-row">
            <span class="card-label">Timezone</span>
            <span class="card-value">{loc().timezone}</span>
          </div>
        </Show>
        <Show when={
          known(loc().registered_country) != null &&
          loc().registered_country_iso !== loc().country_iso
        }>
          <div class="card-row">
            <span class="card-label">Registered</span>
            <span class="card-value">
              {loc().registered_country}
              <Show when={known(loc().registered_country_iso)}>
                {" "}({loc().registered_country_iso})
              </Show>
            </span>
          </div>
        </Show>
      </div>
    </div>
    </>
  );
}
