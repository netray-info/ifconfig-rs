import { Show } from "solid-js";
import type { Ifconfig, SiteMeta } from "../lib/types";
import { showToast } from "../lib/toast";

interface Props {
  data: Ifconfig;
  meta?: SiteMeta | null;
}

function ClipboardIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <rect x="5.5" y="5.5" width="8" height="9" rx="1.5" />
      <path d="M3.5 10.5H3a1.5 1.5 0 0 1-1.5-1.5V3A1.5 1.5 0 0 1 3 1.5h6A1.5 1.5 0 0 1 10.5 3v.5" />
    </svg>
  );
}

export default function IpDisplay(props: Props) {
  const copyIp = async () => {
    try {
      await navigator.clipboard.writeText(props.data.ip.addr);
      showToast("Copied!");
    } catch {
      showToast("Copy failed — clipboard unavailable");
    }
  };

  const copyHost = async () => {
    try {
      await navigator.clipboard.writeText(props.data.ip.hostname!);
      showToast("Copied!");
    } catch {
      showToast("Copy failed — clipboard unavailable");
    }
  };

  return (
    <div class="hero">
      <div class="ip-row">
        <span class="version-badge">IPv{props.data.ip.version}</span>
        <span class={`ip-display${props.data.ip.version === '6' ? ' ip-display--v6' : ''}`}>{props.data.ip.addr}</span>
        <button
          class="copy-icon"
          onClick={copyIp}
          title="Copy IP to clipboard"
          aria-label="Copy IP to clipboard"
        >
          <ClipboardIcon />
        </button>
      </div>
      <Show when={props.data.ip.hostname}>
        <div class="hostname-row">
          <span class="hostname">{props.data.ip.hostname}</span>
          <button
            class="copy-icon"
            onClick={copyHost}
            title="Copy hostname to clipboard"
            aria-label="Copy hostname to clipboard"
          >
            <ClipboardIcon />
          </button>
          <Show when={props.meta?.dns_base_url && props.data.ip.hostname}>
            <a
              class="eco-link eco-link--badge"
              href={`${props.meta!.dns_base_url}/?q=${encodeURIComponent(props.data.ip.hostname!)}&ref=ifconfig`}
              target="_blank"
              rel="noopener noreferrer"
              title={`Check DNS for ${props.data.ip.hostname}`}
            >DNS</a>
          </Show>
          <Show when={props.meta?.tls_base_url && props.data.ip.hostname}>
            <a
              class="eco-link eco-link--badge"
              href={`${props.meta!.tls_base_url}/?h=${encodeURIComponent(props.data.ip.hostname!)}`}
              target="_blank"
              rel="noopener noreferrer"
              title={`Inspect TLS for ${props.data.ip.hostname}`}
            >TLS</a>
          </Show>
        </div>
      </Show>
    </div>
  );
}
