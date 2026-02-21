export interface SiteMeta {
  name: string;
  version: string;
  base_url: string;
  site_name: string;
}

export interface Ifconfig {
  host: Host | null;
  ip: Ip;
  tcp: Tcp;
  location: Location;
  isp: Isp;
  hosting: Hosting | null;
  user_agent: UserAgent | null;
  user_agent_header: string | null;
}

export interface Hosting {
  type: string;
  provider: string | null;
  service: string | null;
  region: string | null;
  is_datacenter: boolean;
  is_vpn: boolean;
  is_tor: boolean;
  is_proxy: boolean;
  is_bot: boolean;
  is_threat: boolean;
}

export interface Host {
  name: string;
}

export interface Ip {
  addr: string;
  version: string;
}

export interface Tcp {
  port: number;
}

export interface Location {
  city: string | null;
  country: string | null;
  country_iso: string | null;
  latitude: number | null;
  longitude: number | null;
  timezone: string | null;
  continent: string | null;
  continent_code: string | null;
  accuracy_radius_km: number | null;
}

export interface Isp {
  name: string | null;
  asn: number | null;
}

export interface UserAgent {
  device: Device;
  os: OS;
  browser: Browser;
}

export interface Device {
  family: string;
  brand: string | null;
  model: string | null;
}

export interface OS {
  family: string;
  major: string | null;
  minor: string | null;
  patch: string | null;
  patch_minor: string | null;
  version: string;
}

export interface Browser {
  family: string;
  major: string | null;
  minor: string | null;
  patch: string | null;
  version: string;
}
