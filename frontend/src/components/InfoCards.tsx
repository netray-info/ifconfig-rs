import { Show } from "solid-js";
import type { Ifconfig } from "../lib/types";

interface Props {
  data: Ifconfig;
}

export default function InfoCards(props: Props) {
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
          <div class="card-row">
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

      {/* Location & ISP Card (merged) */}
      <div class="card card-wide">
        <div class="card-title">Location &amp; ISP</div>
        <div class="card-columns">
          <div class="card-col">
            <Show when={props.data.location.city}>
              <div class="card-row">
                <span class="card-label">City</span>
                <span class="card-value">{props.data.location.city}</span>
              </div>
            </Show>
            <Show when={props.data.location.country}>
              <div class="card-row">
                <span class="card-label">Country</span>
                <span class="card-value">
                  {props.data.location.country}
                  <Show when={props.data.location.country_iso}>
                    {" "}({props.data.location.country_iso})
                  </Show>
                </span>
              </div>
            </Show>
            <Show when={props.data.location.continent}>
              <div class="card-row">
                <span class="card-label">Continent</span>
                <span class="card-value">{props.data.location.continent}</span>
              </div>
            </Show>
            <Show when={props.data.location.timezone}>
              <div class="card-row">
                <span class="card-label">Timezone</span>
                <span class="card-value">{props.data.location.timezone}</span>
              </div>
            </Show>
            <Show
              when={
                props.data.location.latitude != null &&
                props.data.location.longitude != null
              }
            >
              <div class="card-row">
                <span class="card-label">Coordinates</span>
                <span class="card-value">
                  {props.data.location.latitude}, {props.data.location.longitude}
                </span>
              </div>
            </Show>
          </div>
          <div class="card-col">
            <Show when={props.data.isp.name}>
              <div class="card-row">
                <span class="card-label">Provider</span>
                <span class="card-value">{props.data.isp.name}</span>
              </div>
            </Show>
            <Show when={props.data.isp.asn != null}>
              <div class="card-row">
                <span class="card-label">ASN</span>
                <span class="card-value">AS{props.data.isp.asn}</span>
              </div>
            </Show>
          </div>
        </div>
      </div>
    </div>
  );
}
