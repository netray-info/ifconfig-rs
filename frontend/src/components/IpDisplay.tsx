import { createSignal, Show } from "solid-js";
import type { Ifconfig } from "../lib/types";

interface Props {
  data: Ifconfig;
}

function ClipboardIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <rect x="5.5" y="5.5" width="8" height="9" rx="1.5" />
      <path d="M3.5 10.5H3a1.5 1.5 0 0 1-1.5-1.5V3A1.5 1.5 0 0 1 3 1.5h6A1.5 1.5 0 0 1 10.5 3v.5" />
    </svg>
  );
}

function CheckIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <path d="M3 8.5L6.5 12L13 4" />
    </svg>
  );
}

export default function IpDisplay(props: Props) {
  const [copied, setCopied] = createSignal(false);
  const [copiedHost, setCopiedHost] = createSignal(false);

  const copyIp = async () => {
    try {
      await navigator.clipboard.writeText(props.data.ip.addr);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Clipboard API not available
    }
  };

  const copyHost = async () => {
    try {
      await navigator.clipboard.writeText(props.data.host!.name);
      setCopiedHost(true);
      setTimeout(() => setCopiedHost(false), 2000);
    } catch {
      // Clipboard API not available
    }
  };

  return (
    <div class="hero">
      <div class="ip-row">
        <span class="version-badge">IPv{props.data.ip.version}</span>
        <span class="ip-display">{props.data.ip.addr}</span>
        <button
          class={`copy-icon ${copied() ? "copied" : ""}`}
          onClick={copyIp}
          title={copied() ? "Copied!" : "Copy to clipboard"}
        >
          {copied() ? <CheckIcon /> : <ClipboardIcon />}
        </button>
      </div>
      <Show when={props.data.host}>
        <div class="hostname-row">
          <span class="hostname">{props.data.host!.name}</span>
          <button
            class={`copy-icon ${copiedHost() ? "copied" : ""}`}
            onClick={copyHost}
            title={copiedHost() ? "Copied!" : "Copy to clipboard"}
          >
            {copiedHost() ? <CheckIcon /> : <ClipboardIcon />}
          </button>
        </div>
      </Show>
    </div>
  );
}
