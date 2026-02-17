import { createSignal, onMount, Show } from "solid-js";
import type { Ifconfig } from "./lib/types";
import { fetchIfconfig } from "./lib/api";
import IpDisplay from "./components/IpDisplay";
import InfoCards from "./components/InfoCards";
import RequestHeaders from "./components/RequestHeaders";
import ApiExplorer from "./components/ApiExplorer";
import ThemeToggle from "./components/ThemeToggle";

export default function App() {
  const [data, setData] = createSignal<Ifconfig | null>(null);
  const [error, setError] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);

  onMount(async () => {
    try {
      const result = await fetchIfconfig();
      setData(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load");
    } finally {
      setLoading(false);
    }
  });

  return (
    <>
      <ThemeToggle />
      <div class="container">
        <Show when={loading()}>
          <div class="loading">
            <div class="loading-spinner" />
            <div>Loading...</div>
          </div>
        </Show>

        <Show when={error()}>
          <div class="error-msg">{error()}</div>
        </Show>

        <Show when={data()}>
          <IpDisplay data={data()!} />
          <InfoCards data={data()!} />
          <RequestHeaders />
          <ApiExplorer />
        </Show>

        <footer>
          <a href="https://github.com/lukaspustina/ifconfig-rs">ifconfig-rs</a>
          <span class="footer-tagline">IP address lookup service powered by Rust</span>
        </footer>
      </div>
    </>
  );
}
