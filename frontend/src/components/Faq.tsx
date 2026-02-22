import { createSignal, Show, For } from "solid-js";

interface FaqItem {
  q: string;
  a: string;
  link?: { text: string; url: string };
  suffix?: string;
}

function buildFaqItems(siteName: string): FaqItem[] {
  return [
    {
      q: "The IP address is wrong!",
      a: `That's possible. Determining the originating IP address of an HTTP request isn't as easy as it seems. Transparent proxies, load balancers, and NAT gateways between your browser and ${siteName} can hide or alter the real address. If you encounter this, please open an issue on`,
      link: { text: "GitHub", url: "https://github.com/lukaspustina/ifconfig-rs/issues" },
      suffix: " and let's try to enhance the heuristic together.",
    },
    {
      q: "The classification looks wrong!",
      a: "IP classification (VPN, datacenter, Tor, cloud provider, etc.) is based on public CIDR lists and ASN name heuristics that are updated periodically but are never exhaustive. If your IP is misclassified, the relevant data source may be outdated or may not yet cover your provider. Please open an issue on",
      link: { text: "GitHub", url: "https://github.com/lukaspustina/ifconfig-rs/issues" },
      suffix: " with your ASN number and the classification you expected.",
    },
    {
      q: "Can I run my own instance?",
      a: "Absolutely. Just clone or fork the",
      link: { text: "GitHub repository", url: "https://github.com/lukaspustina/ifconfig-rs" },
      suffix: ". See the project's README for details on configuration and deployment.",
    },
    {
      q: "Can you add a feature?",
      a: "Sure, open an issue or send a pull request on",
      link: { text: "GitHub", url: "https://github.com/lukaspustina/ifconfig-rs" },
      suffix: ".",
    },
  ];
}

interface Props {
  siteName: string;
}

export default function Faq(props: Props) {
  const [open, setOpen] = createSignal(false);

  const items = () => buildFaqItems(props.siteName);

  return (
    <div class="section">
      <button class="section-header" onClick={() => setOpen(!open())} aria-expanded={open()} aria-controls="faq-panel">
        <span class={`chevron ${open() ? "open" : ""}`}>&#9654;</span>
        FAQ
      </button>
      <Show when={open()}>
        <div class="faq" id="faq-panel">
          <For each={items()}>
            {(item) => (
              <div class="faq-item">
                <div class="faq-q">{item.q}</div>
                <div class="faq-a">
                  {item.a}
                  <Show when={item.link}>
                    {" "}<a href={item.link!.url} target="_blank" rel="noopener noreferrer">{item.link!.text}</a>
                  </Show>
                  <Show when={item.suffix}>{item.suffix}</Show>
                </div>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
