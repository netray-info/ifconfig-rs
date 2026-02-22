import type { Ifconfig, SiteMeta } from "./types";

function fetchWithTimeout(url: string, init: RequestInit = {}, timeoutMs = 5000): Promise<Response> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  return fetch(url, { ...init, signal: controller.signal }).finally(() => clearTimeout(timer));
}

export async function fetchIfconfig(lang = "en"): Promise<Ifconfig> {
  const url = lang !== "en" ? `/json?lang=${encodeURIComponent(lang)}` : "/json";
  const res = await fetchWithTimeout(url, { headers: { Accept: "application/json" } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchIfconfigForIp(ip: string, lang = "en"): Promise<Ifconfig> {
  let url = `/json?ip=${encodeURIComponent(ip)}`;
  if (lang !== "en") url += `&lang=${encodeURIComponent(lang)}`;
  const res = await fetchWithTimeout(url, { headers: { Accept: "application/json" } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchMeta(): Promise<SiteMeta> {
  const res = await fetchWithTimeout("/meta");
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
