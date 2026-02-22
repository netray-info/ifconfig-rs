import { createSignal, createEffect, on, For, Show } from "solid-js";
import { showToast } from "../lib/toast";

const ENDPOINTS = [
  "/", "/ip", "/tcp", "/location", "/network", "/user_agent", "/headers", "/all", "/ipv4", "/ipv6",
] as const;

const FORMATS = ["plain", "json", "yaml", "toml", "csv"] as const;
type Format = (typeof FORMATS)[number];

function buildUrl(endpoint: string, format: Format, ip?: string | null): string {
  let path: string;
  if (format === "plain") {
    path = endpoint === "/" ? "/" : endpoint;
  } else {
    path = endpoint === "/" ? `/${format}` : `${endpoint}/${format}`;
  }
  return ip ? `${path}?ip=${encodeURIComponent(ip)}` : path;
}

function prettyJson(text: string): string {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
}

function buildCurlCommand(endpoint: string, format: Format, ip?: string | null): string {
  const host = location.hostname === "localhost"
    ? `localhost:${location.port || "8080"}`
    : location.host;
  const path = buildUrl(endpoint, format, ip);
  return `curl ${host}${path}`;
}

function CopySmallIcon() {
  return (
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
      <rect x="5.5" y="5.5" width="8" height="9" rx="1.5" />
      <path d="M3.5 10.5H3a1.5 1.5 0 0 1-1.5-1.5V3A1.5 1.5 0 0 1 3 1.5h6A1.5 1.5 0 0 1 10.5 3v.5" />
    </svg>
  );
}

interface Props {
  lookupIp: string | null;
}

export default function ApiExplorer(props: Props) {
  const [open, setOpen] = createSignal(false);
  const [activeEndpoint, setActiveEndpoint] = createSignal("/");
  const [activeFormat, setActiveFormat] = createSignal<Format>("json");
  const [response, setResponse] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  let currentReqId = 0;

  const copyCurl = async () => {
    try {
      await navigator.clipboard.writeText(buildCurlCommand(activeEndpoint(), activeFormat(), props.lookupIp));
      showToast("Copied!");
    } catch {
      // Clipboard API not available
    }
  };

  createEffect(
    on([activeEndpoint, activeFormat, open, () => props.lookupIp], ([ep, fmt, isOpen, ip]) => {
      if (!isOpen) return;
      const reqId = ++currentReqId;
      setLoading(true);
      setError(null);
      setResponse("");

      const url = buildUrl(ep, fmt, ip as string | null);
      const headers: HeadersInit = fmt === "plain"
        ? { Accept: "text/plain" }
        : {};

      fetch(url, { headers })
        .then((res) => {
          if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
          return res.text();
        })
        .then((text) => {
          if (reqId !== currentReqId) return;
          const display = fmt === "json" ? prettyJson(text) : text;
          setResponse(display);
        })
        .catch((e: unknown) => {
          if (reqId !== currentReqId) return;
          setError(e instanceof Error ? e.message : "Request failed");
        })
        .finally(() => {
          if (reqId === currentReqId) setLoading(false);
        });
    })
  );

  return (
    <div class="section">
      <button class="section-header" onClick={() => setOpen(!open())} aria-expanded={open()} aria-controls="api-explorer-panel">
        <span class={`chevron ${open() ? "open" : ""}`}>&#9654;</span>
        API Explorer
      </button>
      <Show when={open()}>
        <div class="api-explorer" id="api-explorer-panel">
          <div class="endpoint-tabs-wrapper">
            <div class="endpoint-tabs" role="tablist" aria-label="API endpoints">
              <For each={ENDPOINTS as unknown as string[]}>
                {(ep) => (
                  <button
                    class={`endpoint-tab ${activeEndpoint() === ep ? "active" : ""}`}
                    role="tab"
                    aria-selected={activeEndpoint() === ep}
                    tabIndex={activeEndpoint() === ep ? 0 : -1}
                    onClick={() => setActiveEndpoint(ep)}
                    onKeyDown={(e: KeyboardEvent) => {
                      if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
                      const tabs = Array.from(
                        document.querySelectorAll<HTMLButtonElement>(".endpoint-tab")
                      );
                      const idx = tabs.indexOf(e.currentTarget as HTMLButtonElement);
                      if (e.key === "ArrowLeft" && idx > 0) tabs[idx - 1].focus();
                      if (e.key === "ArrowRight" && idx < tabs.length - 1) tabs[idx + 1].focus();
                    }}
                  >
                    {ep}
                  </button>
                )}
              </For>
            </div>
          </div>

          <div class="format-pills">
            <For each={FORMATS as unknown as Format[]}>
              {(fmt) => (
                <button
                  class={`format-pill ${activeFormat() === fmt ? "active" : ""}`}
                  onClick={() => setActiveFormat(fmt)}
                >
                  {fmt}
                </button>
              )}
            </For>
          </div>

          <div class="curl-hint">
            <span
              class="curl-text curl-text-clickable"
              onClick={copyCurl}
              title="Click to copy"
              role="button"
              tabIndex={0}
              onKeyDown={(e: KeyboardEvent) => { if (e.key === "Enter" || e.key === " ") copyCurl(); }}
            >
              <span class="prompt">$ </span>
              {buildCurlCommand(activeEndpoint(), activeFormat(), props.lookupIp)}
            </span>
            <button
              class="curl-copy"
              onClick={copyCurl}
              title="Copy command"
              aria-label="Copy curl command"
            >
              <CopySmallIcon />
            </button>
          </div>

          <div class="response-block">
            <Show when={loading()}>
              <pre class="response-loading">Loading...</pre>
            </Show>
            <Show when={error()}>
              <pre class="response-error">{error()}</pre>
            </Show>
            <Show when={!loading() && !error() && response()}>
              <pre>{response()}</pre>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}
