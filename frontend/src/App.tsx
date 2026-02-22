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

  const loadData = async (ip?: string) => {
    setLoading(true);
    setError(null);
    try {
      const result = ip ? await fetchIfconfigForIp(ip) : await fetchIfconfig();
      setData(result);
      setLookupIp(ip ?? null);
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

    loadData();
  });

  return (
    <>
      <ThemeToggle />
      <div class="container">
        <header class="site-header">
          <h1 class="site-title">{siteName()}</h1>
        </header>

        <IpLookupForm
          onLookup={(ip) => loadData(ip)}
          loading={loading()}
          isLookup={lookupIp() !== null}
          onReset={() => loadData()}
        />

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
          <RequestHeaders />
          <ApiExplorer />
          <Faq siteName={siteName()} />
        </Show>

        <footer>
          <div class="footer-about">
            <em>{siteName()}</em> is a{" "}
            <a href="https://www.google.com/search?q=what's+my+ip+address" target="_blank" rel="noopener noreferrer">"what's my IP address"</a>{" "}
            service running{" "}
            <a href="https://github.com/lukaspustina/ifconfig-rs" target="_blank" rel="noopener noreferrer">ifconfig-rs</a>.
            Written in <a href="https://www.rust-lang.org/" target="_blank" rel="noopener noreferrer">Rust</a>{" "}
            with <a href="https://github.com/tokio-rs/axum" target="_blank" rel="noopener noreferrer">Axum</a>{" "}
            and <a href="https://www.solidjs.com/" target="_blank" rel="noopener noreferrer">SolidJS</a>.
            Includes GeoLite2 data by{" "}
            <a href="https://www.maxmind.com" target="_blank" rel="noopener noreferrer">MaxMind</a>.
            Feel free to use, query, clone, and fork. Rate limiting may apply.
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
