/// ASN-name-based classification for hosting providers and VPN services
/// that lack official machine-readable CIDR lists.
///
/// This module is the single point of change for name-based heuristics.
/// It uses the ASN organization name from MaxMind ASN data (already loaded)
/// and requires no external data files.

#[derive(Debug, Clone, PartialEq)]
pub enum AsnClassification {
    Hosting { provider: &'static str },
    Vpn { provider: &'static str },
    None,
}

/// Classify an ASN based on its organization name.
///
/// Matching is case-insensitive and uses substring matching against known
/// provider patterns. Returns the first match found.
pub fn classify_asn(asn_org: &str) -> AsnClassification {
    let lower = asn_org.to_ascii_lowercase();

    // Hosting / datacenter providers without official CIDR lists
    for &(pattern, provider) in HOSTING_PATTERNS {
        if lower.contains(pattern) {
            return AsnClassification::Hosting { provider };
        }
    }

    // VPN providers (fallback for those not covered by X4BNet CIDR lists)
    for &(pattern, provider) in VPN_PATTERNS {
        if lower.contains(pattern) {
            return AsnClassification::Vpn { provider };
        }
    }

    AsnClassification::None
}

static HOSTING_PATTERNS: &[(&str, &str)] = &[
    // Named providers (specific matches first)
    ("google llc", "Google"),
    ("hetzner", "Hetzner"),
    ("digitalocean", "DigitalOcean"),
    ("ovh", "OVH"),
    ("online s.a.s.", "Scaleway"),
    ("scaleway", "Scaleway"),
    ("vultr", "Vultr"),
    ("choopa", "Vultr"),
    ("linode", "Linode"),
    ("akamai connected cloud", "Linode"),
    ("akamai", "Akamai"),
    ("contabo", "Contabo"),
    ("ionos", "IONOS"),
    ("leaseweb", "Leaseweb"),
    ("hostinger", "Hostinger"),
    ("kamatera", "Kamatera"),
    ("upcloud", "UpCloud"),
    ("cherry servers", "Cherry Servers"),
    ("equinix", "Equinix"),
    ("rackspace", "Rackspace"),
    ("softlayer", "IBM Cloud"),
    ("ibm cloud", "IBM Cloud"),
    ("oracle", "Oracle Cloud"),
    ("alibaba", "Alibaba Cloud"),
    ("tencent cloud", "Tencent Cloud"),
    ("fastly", "Fastly"),
    ("cloudflare", "Cloudflare"),
    ("zscaler", "Zscaler"),
    ("godaddy", "GoDaddy"),
    ("dreamhost", "DreamHost"),
    ("colocrossing", "ColoCrossing"),
    ("quadranet", "QuadraNet"),
    ("psychz", "Psychz Networks"),
    ("phoenixnap", "PhoenixNAP"),
    ("m247", "M247"),
    ("zenlayer", "Zenlayer"),
    ("sharktech", "Sharktech"),
    // Generic keyword patterns (catch-all, last so named providers win)
    ("hosting", "hosting provider"),
    ("datacenter", "datacenter"),
    ("data center", "datacenter"),
    ("colocation", "colocation"),
    ("dedicated server", "dedicated hosting"),
];

static VPN_PATTERNS: &[(&str, &str)] = &[
    ("mullvad", "Mullvad"),
    ("31173 services", "Mullvad"),
    ("nordvpn", "NordVPN"),
    ("expressvpn", "ExpressVPN"),
    ("surfshark", "Surfshark"),
    ("cyberghost", "CyberGhost"),
    ("private internet access", "PIA"),
    ("proton ag", "ProtonVPN"),
    ("proton vpn", "ProtonVPN"),
    ("ipvanish", "IPVanish"),
    ("hide.me", "hide.me"),
    ("windscribe", "Windscribe"),
    ("astrill", "Astrill"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_hetzner() {
        assert_eq!(
            classify_asn("Hetzner Online GmbH"),
            AsnClassification::Hosting { provider: "Hetzner" }
        );
    }

    #[test]
    fn classify_digitalocean() {
        assert_eq!(
            classify_asn("DIGITALOCEAN-ASN"),
            AsnClassification::Hosting { provider: "DigitalOcean" }
        );
    }

    #[test]
    fn classify_mullvad() {
        assert_eq!(
            classify_asn("MULLVAD-VPN-SE"),
            AsnClassification::Vpn { provider: "Mullvad" }
        );
    }

    #[test]
    fn classify_mullvad_31173() {
        assert_eq!(
            classify_asn("31173 Services AB"),
            AsnClassification::Vpn { provider: "Mullvad" }
        );
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(classify_asn("Deutsche Telekom AG"), AsnClassification::None);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(
            classify_asn("hetzner online gmbh"),
            AsnClassification::Hosting { provider: "Hetzner" }
        );
    }

    #[test]
    fn classify_vultr_choopa() {
        assert_eq!(
            classify_asn("AS-CHOOPA - Choopa, LLC"),
            AsnClassification::Hosting { provider: "Vultr" }
        );
    }

    #[test]
    fn classify_linode_akamai() {
        assert_eq!(
            classify_asn("Akamai Connected Cloud"),
            AsnClassification::Hosting { provider: "Linode" }
        );
    }

    #[test]
    fn classify_akamai_technologies() {
        assert_eq!(
            classify_asn("Akamai Technologies, Inc."),
            AsnClassification::Hosting { provider: "Akamai" }
        );
    }

    #[test]
    fn classify_oracle_corporation() {
        assert_eq!(
            classify_asn("Oracle Corporation"),
            AsnClassification::Hosting {
                provider: "Oracle Cloud"
            }
        );
    }

    #[test]
    fn classify_cloudflare() {
        assert_eq!(
            classify_asn("CLOUDFLARENET"),
            AsnClassification::Hosting {
                provider: "Cloudflare"
            }
        );
    }

    #[test]
    fn classify_generic_hosting() {
        assert_eq!(
            classify_asn("Example Hosting Ltd"),
            AsnClassification::Hosting {
                provider: "hosting provider"
            }
        );
    }

    #[test]
    fn classify_generic_datacenter() {
        assert_eq!(
            classify_asn("XYZ Datacenter GmbH"),
            AsnClassification::Hosting {
                provider: "datacenter"
            }
        );
    }

    #[test]
    fn classify_google_llc() {
        assert_eq!(
            classify_asn("Google LLC"),
            AsnClassification::Hosting { provider: "Google" }
        );
    }

    #[test]
    fn no_false_positive_google_fiber() {
        assert_eq!(classify_asn("Google Fiber Inc."), AsnClassification::None);
    }

    #[test]
    fn classify_protonvpn() {
        assert_eq!(
            classify_asn("Proton AG"),
            AsnClassification::Vpn { provider: "ProtonVPN" }
        );
    }
}
