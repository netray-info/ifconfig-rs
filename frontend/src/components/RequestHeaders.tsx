import { createSignal, createEffect, Show, For } from "solid-js";

export default function RequestHeaders() {
  const [open, setOpen] = createSignal(false);
  const [headers, setHeaders] = createSignal<[string, string][]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let fetched = false;

  createEffect(() => {
    if (!open() || fetched) return;
    fetched = true;
    setLoading(true);
    fetch("/headers/json", { headers: { Accept: "application/json" } })
      .then((res) => {
        if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
        return res.json();
      })
      .then((data: Record<string, string>) => {
        setHeaders(Object.entries(data).sort(([a], [b]) => a.localeCompare(b)));
      })
      .catch((e) => {
        setError(e instanceof Error ? e.message : "Request failed");
      })
      .finally(() => {
        setLoading(false);
      });
  });

  return (
    <div class="section">
      <button class="section-header" onClick={() => setOpen(!open())} aria-expanded={open()} aria-controls="request-headers-panel">
        <span class={`chevron ${open() ? "open" : ""}`}>&#9654;</span>
        Request Headers
      </button>
      <Show when={open()}>
        <div class="headers-card" id="request-headers-panel">
          <Show when={loading()}>
            <div class="headers-loading">Loading...</div>
          </Show>
          <Show when={error()}>
            <div class="headers-error">{error()}</div>
          </Show>
          <Show when={!loading() && !error() && headers().length > 0}>
            <For each={headers()}>
              {([name, value]) => (
                <div class="header-row">
                  <span class="header-name">{name}</span>
                  <span class="header-value">{value}</span>
                </div>
              )}
            </For>
          </Show>
        </div>
      </Show>
    </div>
  );
}
