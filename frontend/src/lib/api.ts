import type { Ifconfig, SiteMeta } from "./types";

export async function fetchIfconfig(): Promise<Ifconfig> {
  const res = await fetch("/json", {
    headers: { Accept: "application/json" },
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchMeta(): Promise<SiteMeta> {
  const res = await fetch("/meta");
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
