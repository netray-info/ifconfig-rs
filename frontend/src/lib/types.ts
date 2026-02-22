export interface SiteMeta {
  name: string;
  version: string;
  base_url: string;
  site_name: string;
}

export interface Ifconfig {
  ip: Ip;
  tcp: Tcp | null;
  location: Location;
  network: Network;
  user_agent: UserAgent | null;
}

export interface Classification {
  type: string;
  is_datacenter: boolean;
  is_vpn: boolean;
  is_tor: boolean;
  is_proxy: boolean;
  is_bot: boolean;
  is_threat: boolean;
}

export interface Network {
  asn: number | null;
  org: string | null;
  prefix: string | null;
  provider: string | null;
  service: string | null;
  region: string | null;
  classification: Classification;
}

export interface Ip {
  addr: string;
  version: string;
  hostname: string | null;
}

export interface Tcp {
  port: number;
}

export interface Location {
  city: string | null;
  region: string | null;
  region_code: string | null;
  country: string | null;
  country_iso: string | null;
  postal_code: string | null;
  is_eu: boolean | null;
  latitude: number | null;
  longitude: number | null;
  timezone: string | null;
  continent: string | null;
  continent_code: string | null;
  accuracy_radius_km: number | null;
  registered_country: string | null;
  registered_country_iso: string | null;
}


export interface UserAgent {
  raw: string | null;
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
