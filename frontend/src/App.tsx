import { createSignal, onMount, Show } from "solid-js";
import type { Ifconfig, SiteMeta } from "./lib/types";
import { fetchIfconfig, fetchIfconfigForIp, fetchMeta } from "./lib/api";
import { showToast, toastMessage } from "./lib/toast";
import { createTheme } from "@netray-info/common-frontend/theme";
import ThemeToggle from "@netray-info/common-frontend/components/ThemeToggle";
import SiteFooter from "@netray-info/common-frontend/components/SiteFooter";
import Modal from "@netray-info/common-frontend/components/Modal";
import IpDisplay from "./components/IpDisplay";
import InfoCards from "./components/InfoCards";
import RequestHeaders from "./components/RequestHeaders";
import ApiExplorer from "./components/ApiExplorer";
import Faq from "./components/Faq";
import IpLookupForm from "./components/IpLookupForm";
import SuiteNav from '@netray-info/common-frontend/components/SuiteNav';

export default function App() {
  const themeResult = createTheme('ifconfig_theme', 'system');
  const [showHelp, setShowHelp] = createSignal(false);

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
      <SuiteNav current="ip" meta={meta() ?? undefined} />
      <div class="header-actions">
        <ThemeToggle theme={themeResult} class="header-btn" />
        <button
          class="header-btn"
          onClick={() => setShowHelp(true)}
          aria-label="Open help"
          title="Help"
        >?</button>
      </div>
      <div class="container">
        <header class="site-header">
          <h1 class="site-title">
            {siteName()}
            <button
              class="share-icon"
              title="Share this IP lookup"
              onClick={async () => {
                const url = lookupIp()
                  ? `${location.origin}/?ip=${lookupIp()}`
                  : location.href;
                if (navigator.share) {
                  try {
                    await navigator.share({ url });
                  } catch {
                    // user cancelled — ignore
                  }
                } else {
                  await navigator.clipboard.writeText(url);
                  showToast("Link copied to clipboard");
                }
              }}
            >
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="5" r="3"/><circle cx="6" cy="12" r="3"/><circle cx="18" cy="19" r="3"/><line x1="8.59" y1="13.51" x2="15.42" y2="17.49"/><line x1="15.41" y1="6.51" x2="8.59" y2="10.49"/></svg>
            </button>
          </h1>
          <span class="tagline">IP, decoded</span>
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
          <div class="error-msg" role="alert">
            {error()}
            {" "}
            <button class="retry-btn" onClick={() => loadData(lookupIp() ?? undefined)}>Try again</button>
          </div>
        </Show>

        <Show when={data()}>
          <IpDisplay data={data()!} meta={meta()} />
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

        <SiteFooter
          aboutText={<>
            <em>{siteName()}</em> is an IP address and network information service.
            Returns geolocation, ASN, and user-agent details via a plain HTTP API.
            Built in <a href="https://www.rust-lang.org/" target="_blank" rel="noopener noreferrer">Rust</a>{" "}
            with <a href="https://github.com/tokio-rs/axum" target="_blank" rel="noopener noreferrer">Axum</a>{" "}
            and <a href="https://www.solidjs.com/" target="_blank" rel="noopener noreferrer">SolidJS</a>,{" "}
            powered by <a href="https://github.com/lukaspustina/ifconfig-rs" target="_blank" rel="noopener noreferrer">ifconfig-rs</a>.
            Geolocation data by{" "}
            <a href="https://www.maxmind.com" target="_blank" rel="noopener noreferrer">MaxMind</a> GeoLite2.
            Open to use and self-host — rate limiting applies.
          </>}
          links={[
            { href: "https://github.com/lukaspustina/ifconfig-rs", label: "GitHub", external: true },
            { href: "/docs", label: "API Docs" },
            { href: "https://lukas.pustina.de", label: "Author", external: true },
          ]}
          version={meta()?.version}
        />
      </div>

      <Modal open={showHelp()} onClose={() => setShowHelp(false)} title="Help">
        <div class="help-section">
          <div class="help-section__title">About</div>
          <p class="help-desc">
            {siteName()} shows your public IP address along with geolocation, ASN, network type, and user agent details.
            You can look up any IP address using the lookup form.
          </p>
        </div>
        <div class="help-section">
          <div class="help-section__title">IP Lookup</div>
          <p class="help-desc">Enter any IPv4 or IPv6 address in the lookup form to see its enrichment data. The URL updates so you can bookmark or share the result.</p>
        </div>
      </Modal>

      <Show when={toastMessage()}>
        <div class="toast" role="status" aria-live="polite">
          {toastMessage()}
        </div>
      </Show>
    </>
  );
}
