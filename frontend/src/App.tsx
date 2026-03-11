import { createSignal, onMount, Show } from "solid-js";
import type { Ifconfig, SiteMeta } from "./lib/types";
import { fetchIfconfig, fetchIfconfigForIp, fetchMeta } from "./lib/api";
import { toastMessage } from "./lib/toast";
import IpDisplay from "./components/IpDisplay";
import InfoCards from "./components/InfoCards";
import RequestHeaders from "./components/RequestHeaders";
import ApiExplorer from "./components/ApiExplorer";
import Faq from "./components/Faq";
import ThemeToggle from "./components/ThemeToggle";
import IpLookupForm from "./components/IpLookupForm";

export default function App() {
  const [data, setData] = createSignal<Ifconfig | null>(null);
  const [meta, setMeta] = createSignal<SiteMeta | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [lookupIp, setLookupIp] = createSignal<string | null>(null);

  const siteName = () => meta()?.site_name ?? location.hostname;

  const loadData = async (ip?: string, pushState = true) => {
    setLoading(true);
    setError(null);
    try {
      const result = ip ? await fetchIfconfigForIp(ip) : await fetchIfconfig();
      setData(result);
      setLookupIp(ip ?? null);
      if (pushState) {
        const url = ip ? `/?ip=${ip}` : "/";
        history.pushState({ ip: ip ?? null }, "", url);
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load");
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    // Fetch meta independently — failure should not block the main data load
    fetchMeta()
      .then((siteMeta) => {
        setMeta(siteMeta);
        document.title = siteMeta.site_name;
      })
      .catch(() => {}); // fall back to location.hostname via siteName()

    // Read ?ip= from URL on initial load (bookmarkable / shareable links)
    const initialIp = new URLSearchParams(location.search).get("ip") ?? undefined;
    loadData(initialIp, false);

    // Keep URL in sync with browser back/forward
    window.addEventListener("popstate", (e) => {
      const ip = (e.state as { ip?: string | null } | null)?.ip ?? undefined;
      loadData(ip, false);
    });
  });

  return (
    <>
      <ThemeToggle />
      <div class="container">
        <header class="site-header">
          <h1 class="site-title">{siteName()}</h1>
        </header>

        <Show when={loading() && !data()}>
          <div role="status" aria-label="Loading your IP information">
            <div class="skeleton-hero skeleton-block" />
            <div class="skeleton-subtitle skeleton-block" />
            <div class="skeleton-cards">
              <div class="skeleton-card">
                <div class="skeleton-card-title skeleton-block" />
                <div class="skeleton-card-row skeleton-block" />
                <div class="skeleton-card-row skeleton-block" />
                <div class="skeleton-card-row-short skeleton-block" />
              </div>
              <div class="skeleton-card">
                <div class="skeleton-card-title skeleton-block" />
                <div class="skeleton-card-row skeleton-block" />
                <div class="skeleton-card-row skeleton-block" />
                <div class="skeleton-card-row-short skeleton-block" />
              </div>
              <div class="skeleton-card">
                <div class="skeleton-card-title skeleton-block" />
                <div class="skeleton-card-row skeleton-block" />
                <div class="skeleton-card-row skeleton-block" />
                <div class="skeleton-card-row-short skeleton-block" />
              </div>
            </div>
          </div>
        </Show>

        <Show when={error()}>
          <div class="error-msg">
            {error()}
            {" "}
            <button class="retry-btn" onClick={() => loadData(lookupIp() ?? undefined)}>Try again</button>
          </div>
        </Show>

        <Show when={data()}>
          <IpDisplay data={data()!} />
          <InfoCards data={data()!} />
          <IpLookupForm
            onLookup={(ip) => loadData(ip)}
            loading={loading()}
            isLookup={lookupIp() !== null}
            onReset={() => loadData()}
            value={lookupIp()}
          />
          <RequestHeaders />
          <ApiExplorer lookupIp={lookupIp()} />
          <Faq siteName={siteName()} />
        </Show>

        <footer>
          <div class="footer-about">
            <em>{siteName()}</em> is an IP address and network information service.
            Returns geolocation, ASN, and user-agent details via a plain HTTP API.
            Built in <a href="https://www.rust-lang.org/" target="_blank" rel="noopener noreferrer">Rust</a>{" "}
            with <a href="https://github.com/tokio-rs/axum" target="_blank" rel="noopener noreferrer">Axum</a>{" "}
            and <a href="https://www.solidjs.com/" target="_blank" rel="noopener noreferrer">SolidJS</a>,{" "}
            powered by <a href="https://github.com/lukaspustina/ifconfig-rs" target="_blank" rel="noopener noreferrer">ifconfig-rs</a>.
            Geolocation data by{" "}
            <a href="https://www.maxmind.com" target="_blank" rel="noopener noreferrer">MaxMind</a> GeoLite2.
            Open to use and self-host — rate limiting applies.
          </div>
          <div class="footer-links">
            <a href="https://github.com/lukaspustina/ifconfig-rs" target="_blank" rel="noopener noreferrer">GitHub</a>
            {" · "}
            <a href="/docs">API Docs</a>
            {" · "}
            <a href="https://lukas.pustina.de" target="_blank" rel="noopener noreferrer">Author</a>
            <Show when={meta()?.version}>
              {" · "}
              <span class="footer-version">v{meta()!.version}</span>
            </Show>
          </div>
        </footer>
      </div>

      <Show when={toastMessage()}>
        <div class="toast" role="status" aria-live="polite">
          {toastMessage()}
        </div>
      </Show>
    </>
  );
}
