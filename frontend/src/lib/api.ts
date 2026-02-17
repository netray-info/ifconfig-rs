import type { Ifconfig } from "./types";

export async function fetchIfconfig(): Promise<Ifconfig> {
  const res = await fetch("/json", {
    headers: { Accept: "application/json" },
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
