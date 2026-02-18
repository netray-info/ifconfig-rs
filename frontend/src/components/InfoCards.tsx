import { Show } from "solid-js";
import type { Ifconfig } from "../lib/types";

interface Props {
  data: Ifconfig;
}

/** Treat null, undefined, and "unknown" as missing. */
function known(v: string | null | undefined): string | null {
  return v && v !== "unknown" ? v : null;
}

/** Convert decimal degrees to DMS string, e.g. 50.0471 lat → "50°02'49.6"N" */
function toDMS(deg: number, isLat: boolean): string {
  const abs = Math.abs(deg);
  const d = Math.floor(abs);
  const mFull = (abs - d) * 60;
  const m = Math.floor(mFull);
  const s = (mFull - m) * 60;
  const dir = isLat ? (deg >= 0 ? "N" : "S") : (deg >= 0 ? "E" : "W");
  return `${d}°${String(m).padStart(2, "0")}'${s.toFixed(1)}"${dir}`;
}

export default function InfoCards(props: Props) {
  const loc = () => props.data.location;
  const isp = () => props.data.isp;

  const mapsUrl = () => {
    const { latitude, longitude } = loc();
    if (latitude == null || longitude == null) return null;
    return `https://www.google.com/maps/place/${encodeURIComponent(toDMS(latitude, true))}+${encodeURIComponent(toDMS(longitude, false))}`;
  };

  return (
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
        <div class="card-row">
          <span class="card-label">TCP Port</span>
          <span class="card-value">{props.data.tcp.port}</span>
        </div>
        <Show when={props.data.host}>
          <div class="card-row card-row-stackable">
            <span class="card-label">Hostname</span>
            <span class="card-value">{props.data.host!.name}</span>
          </div>
        </Show>
        <Show when={props.data.is_tor != null}>
          <div class="card-row">
            <span class="card-label">Tor Exit Node</span>
            <span class="card-value">
              <span class={`tor-badge ${props.data.is_tor ? "tor" : "safe"}`}>
                {props.data.is_tor ? "yes" : "no"}
              </span>
            </span>
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

      {/* Location & ISP Card */}
      <div class="card">
        <div class="card-title">Location &amp; ISP</div>
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
        <Show when={known(isp().name)}>
          <div class="card-row">
            <span class="card-label">Provider</span>
            <span class="card-value">{isp().name}</span>
          </div>
        </Show>
        <Show when={isp().asn != null}>
          <div class="card-row">
            <span class="card-label">ASN</span>
            <span class="card-value">AS{isp().asn}</span>
          </div>
        </Show>
      </div>
    </div>
  );
}
