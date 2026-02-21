import { createSignal, onMount, Show } from "solid-js";
import type { Ifconfig, SiteMeta } from "./lib/types";
import { fetchIfconfig, fetchMeta } from "./lib/api";
import IpDisplay from "./components/IpDisplay";
import InfoCards from "./components/InfoCards";
import RequestHeaders from "./components/RequestHeaders";
import ApiExplorer from "./components/ApiExplorer";
import Faq from "./components/Faq";
import ThemeToggle from "./components/ThemeToggle";

export default function App() {
  const [data, setData] = createSignal<Ifconfig | null>(null);
  const [meta, setMeta] = createSignal<SiteMeta | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);

  const siteName = () => meta()?.site_name ?? location.hostname;

  const loadData = async () => {
    setLoading(true);
    setError(null);
    try {
      setData(await fetchIfconfig());
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

        <Show when={loading()}>
          <div class="loading" role="status" aria-label="Loading your IP information">
            <div class="loading-spinner" />
            <div>Loading...</div>
          </div>
        </Show>

        <Show when={error()}>
          <div class="error-msg">
            {error()}
            {" "}
            <button class="retry-btn" onClick={loadData}>Try again</button>
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
    </>
  );
}
