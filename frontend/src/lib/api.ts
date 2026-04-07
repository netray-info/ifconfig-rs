import type { Ifconfig, SiteMeta } from "./types";
import { fetchWithTimeout } from '@netray-info/common-frontend/api';

export async function fetchIfconfig(): Promise<Ifconfig> {
  const res = await fetchWithTimeout("/json", { headers: { Accept: "application/json" } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchIfconfigForIp(ip: string): Promise<Ifconfig> {
  const url = `/json?ip=${ip}`;
  const res = await fetchWithTimeout(url, { headers: { Accept: "application/json" } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function fetchMeta(): Promise<SiteMeta> {
  const res = await fetchWithTimeout("/meta");
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}
