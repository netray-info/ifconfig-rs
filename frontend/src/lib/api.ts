import type { Ifconfig, SiteMeta } from "./types";

function fetchWithTimeout(url: string, init: RequestInit = {}, timeoutMs = 5000): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  return fetch(url, { ...init, signal: controller.signal }).finally(() => clearTimeout(timer));
}

export async function fetchIfconfig(): Promise<Ifconfig> {
  const res = await fetchWithTimeout("/json", { headers: { Accept: "application/json" } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchIfconfigForIp(ip: string): Promise<Ifconfig> {
  const url = `/json?ip=${encodeURIComponent(ip)}`;
  const res = await fetchWithTimeout(url, { headers: { Accept: "application/json" } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchMeta(): Promise<SiteMeta> {
  const res = await fetchWithTimeout("/meta");
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
