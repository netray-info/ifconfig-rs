import { createSignal, Show } from "solid-js";

interface Props {
  onLookup: (ip: string) => void;
  loading: boolean;
  isLookup: boolean;
  onReset: () => void;
}

export default function IpLookupForm(props: Props) {
  const [input, setInput] = createSignal("");

  const handleSubmit = (e: SubmitEvent) => {
    e.preventDefault();
    const ip = input().trim();
    if (!ip) return;
    props.onLookup(ip);
  };

  return (
    <div class="lookup-row">
      <Show when={props.isLookup}>
        <button class="lookup-reset" type="button" onClick={props.onReset} title="Back to my IP">
          ← My IP
        </button>
      </Show>
      <form class="lookup-form" onSubmit={handleSubmit}>
        <input
          class="lookup-input"
          type="text"
          placeholder="Look up an IP address…"
          value={input()}
          onInput={(e) => setInput(e.currentTarget.value)}
          disabled={props.loading}
          autocomplete="off"
          spellcheck={false}
        />
        <button class="lookup-btn" type="submit" disabled={props.loading || !input().trim()}>
          Look up
        </button>
      </form>
    </div>
  );
}
