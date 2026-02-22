/// ASN-name-based classification for hosting providers and VPN services
/// that lack official machine-readable CIDR lists.
///
/// Patterns are loaded from an external TOML file when configured, falling
/// back to compiled-in defaults. Matching uses case-insensitive regex against
/// the ASN organization name from MaxMind ASN data, or exact ASN number match.
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub enum AsnClassification {
    Hosting { provider: String },
    Vpn { provider: String },
    None,
}

#[derive(Debug, Deserialize)]
struct RawPatternEntry {
    provider: String,
    #[serde(default)]
    asn: Option<u32>,
    #[serde(default)]
    pattern: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct RawAsnPatterns {
    #[serde(default)]
    hosting: Vec<RawPatternEntry>,
    #[serde(default)]
    vpn: Vec<RawPatternEntry>,
}

pub struct CompiledEntry {
    pub provider: String,
    pub asn: Option<u32>,
    pub pattern: Option<Regex>,
}

/// Compiled ASN classification patterns. Always present — either loaded from
/// a file or constructed from the built-in defaults.
pub struct AsnPatterns {
    pub hosting: Vec<CompiledEntry>,
    pub vpn: Vec<CompiledEntry>,
}

impl AsnPatterns {
    /// Load patterns from a TOML file. Returns `Err` on I/O error, parse error,
    /// or invalid regex.
    pub async fn from_file(path: &str) -> Result<Self, String> {
        let contents = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("failed to read {path}: {e}"))?;
        let raw: RawAsnPatterns = toml::from_str(&contents)
            .map_err(|e| format!("failed to parse {path}: {e}"))?;
        let hosting = compile_patterns(raw.hosting)?;
        let vpn = compile_patterns(raw.vpn)?;
        Ok(AsnPatterns { hosting, vpn })
    }

    /// Build patterns from the built-in static arrays. Panics if a built-in
    /// pattern is invalid (which is a compile-time invariant).
    pub fn builtin() -> Self {
        let hosting = HOSTING_PATTERNS
            .iter()
            .map(|&(pat, provider)| CompiledEntry {
                provider: provider.to_string(),
                asn: None,
                pattern: Some(Regex::new(pat).expect("built-in hosting pattern is valid")),
            })
            .chain(HOSTING_ASN.iter().map(|&(asn, provider)| CompiledEntry {
                provider: provider.to_string(),
                asn: Some(asn),
                pattern: None,
            }))
            .collect();
        let vpn = VPN_PATTERNS
            .iter()
            .map(|&(pat, provider)| CompiledEntry {
                provider: provider.to_string(),
                asn: None,
                pattern: Some(Regex::new(pat).expect("built-in VPN pattern is valid")),
            })
            .chain(VPN_ASN.iter().map(|&(asn, provider)| CompiledEntry {
                provider: provider.to_string(),
                asn: Some(asn),
                pattern: None,
            }))
            .collect();
        AsnPatterns { hosting, vpn }
    }
}

fn compile_patterns(entries: Vec<RawPatternEntry>) -> Result<Vec<CompiledEntry>, String> {
    entries
        .into_iter()
        .map(|entry| {
            if entry.asn.is_none() && entry.pattern.is_none() {
                return Err(format!(
                    "entry for {:?} has neither `asn` nor `pattern`",
                    entry.provider
                ));
            }
            let pattern = entry
                .pattern
                .map(|p| Regex::new(&p).map_err(|e| format!("invalid pattern {:?}: {e}", p)))
                .transpose()?;
            Ok(CompiledEntry {
                provider: entry.provider,
                asn: entry.asn,
                pattern,
            })
        })
        .collect()
}

/// Classify an ASN based on its number and/or organization name.
///
/// Pass 1: exact ASN number match (higher confidence, skipped if `asn_number` is `None`).
/// Pass 2: case-insensitive regex match on org name (skipped if `asn_org` is `None`).
/// Returns the first match found across both passes.
pub fn classify_asn(
    asn_number: Option<u32>,
    asn_org: Option<&str>,
    patterns: &AsnPatterns,
) -> AsnClassification {
    // Pass 1: ASN number matches
    if let Some(n) = asn_number {
        for entry in &patterns.hosting {
            if entry.asn == Some(n) {
                return AsnClassification::Hosting {
                    provider: entry.provider.clone(),
                };
            }
        }
        for entry in &patterns.vpn {
            if entry.asn == Some(n) {
                return AsnClassification::Vpn {
                    provider: entry.provider.clone(),
                };
            }
        }
    }

    // Pass 2: org-name pattern matches
    if let Some(org) = asn_org {
        let lower = org.to_ascii_lowercase();
        for entry in &patterns.hosting {
            if entry.pattern.as_ref().is_some_and(|re| re.is_match(&lower)) {
                return AsnClassification::Hosting {
                    provider: entry.provider.clone(),
                };
            }
        }
        for entry in &patterns.vpn {
            if entry.pattern.as_ref().is_some_and(|re| re.is_match(&lower)) {
                return AsnClassification::Vpn {
                    provider: entry.provider.clone(),
                };
            }
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

static HOSTING_ASN: &[(u32, &str)] = &[];

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

static VPN_ASN: &[(u32, &str)] = &[];

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin() -> AsnPatterns {
        AsnPatterns::builtin()
    }

    #[test]
    fn classify_hetzner() {
        assert_eq!(
            classify_asn(None, Some("Hetzner Online GmbH"), &builtin()),
            AsnClassification::Hosting { provider: "Hetzner".to_string() }
        );
    }

    #[test]
    fn classify_digitalocean() {
        assert_eq!(
            classify_asn(None, Some("DIGITALOCEAN-ASN"), &builtin()),
            AsnClassification::Hosting { provider: "DigitalOcean".to_string() }
        );
    }

    #[test]
    fn classify_mullvad() {
        assert_eq!(
            classify_asn(None, Some("MULLVAD-VPN-SE"), &builtin()),
            AsnClassification::Vpn { provider: "Mullvad".to_string() }
        );
    }

    #[test]
    fn classify_mullvad_31173() {
        assert_eq!(
            classify_asn(None, Some("31173 Services AB"), &builtin()),
            AsnClassification::Vpn { provider: "Mullvad".to_string() }
        );
    }

    #[test]
    fn classify_unknown() {
        assert_eq!(classify_asn(None, Some("Deutsche Telekom AG"), &builtin()), AsnClassification::None);
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(
            classify_asn(None, Some("hetzner online gmbh"), &builtin()),
            AsnClassification::Hosting { provider: "Hetzner".to_string() }
        );
    }

    #[test]
    fn classify_vultr_choopa() {
        assert_eq!(
            classify_asn(None, Some("AS-CHOOPA - Choopa, LLC"), &builtin()),
            AsnClassification::Hosting { provider: "Vultr".to_string() }
        );
    }

    #[test]
    fn classify_linode_akamai() {
        assert_eq!(
            classify_asn(None, Some("Akamai Connected Cloud"), &builtin()),
            AsnClassification::Hosting { provider: "Linode".to_string() }
        );
    }

    #[test]
    fn classify_akamai_technologies() {
        assert_eq!(
            classify_asn(None, Some("Akamai Technologies, Inc."), &builtin()),
            AsnClassification::Hosting { provider: "Akamai".to_string() }
        );
    }

    #[test]
    fn classify_oracle_corporation() {
        assert_eq!(
            classify_asn(None, Some("Oracle Corporation"), &builtin()),
            AsnClassification::Hosting {
                provider: "Oracle Cloud".to_string()
            }
        );
    }

    #[test]
    fn classify_cloudflare() {
        assert_eq!(
            classify_asn(None, Some("CLOUDFLARENET"), &builtin()),
            AsnClassification::Hosting {
                provider: "Cloudflare".to_string()
            }
        );
    }

    #[test]
    fn classify_generic_hosting() {
        assert_eq!(
            classify_asn(None, Some("Example Hosting Ltd"), &builtin()),
            AsnClassification::Hosting {
                provider: "hosting provider".to_string()
            }
        );
    }

    #[test]
    fn classify_generic_datacenter() {
        assert_eq!(
            classify_asn(None, Some("XYZ Datacenter GmbH"), &builtin()),
            AsnClassification::Hosting {
                provider: "datacenter".to_string()
            }
        );
    }

    #[test]
    fn classify_google_llc() {
        assert_eq!(
            classify_asn(None, Some("Google LLC"), &builtin()),
            AsnClassification::Hosting { provider: "Google".to_string() }
        );
    }

    #[test]
    fn no_false_positive_google_fiber() {
        assert_eq!(classify_asn(None, Some("Google Fiber Inc."), &builtin()), AsnClassification::None);
    }

    #[test]
    fn classify_protonvpn() {
        assert_eq!(
            classify_asn(None, Some("Proton AG"), &builtin()),
            AsnClassification::Vpn { provider: "ProtonVPN".to_string() }
        );
    }

    #[test]
    fn classify_by_asn_hosting() {
        let patterns = AsnPatterns {
            hosting: vec![CompiledEntry {
                provider: "TestHost".to_string(),
                asn: Some(64496),
                pattern: None,
            }],
            vpn: vec![],
        };
        assert_eq!(
            classify_asn(Some(64496), None, &patterns),
            AsnClassification::Hosting { provider: "TestHost".to_string() }
        );
    }

    #[test]
    fn classify_by_asn_vpn() {
        let patterns = AsnPatterns {
            hosting: vec![],
            vpn: vec![CompiledEntry {
                provider: "TestVPN".to_string(),
                asn: Some(64497),
                pattern: None,
            }],
        };
        assert_eq!(
            classify_asn(Some(64497), None, &patterns),
            AsnClassification::Vpn { provider: "TestVPN".to_string() }
        );
    }

    #[test]
    fn asn_beats_pattern_miss() {
        // ASN matches, no org supplied — should return Hosting without needing org
        let patterns = AsnPatterns {
            hosting: vec![CompiledEntry {
                provider: "TestHost".to_string(),
                asn: Some(64496),
                pattern: None,
            }],
            vpn: vec![],
        };
        assert_eq!(
            classify_asn(Some(64496), None, &patterns),
            AsnClassification::Hosting { provider: "TestHost".to_string() }
        );
    }

    #[test]
    fn none_when_both_absent() {
        assert_eq!(classify_asn(None, None, &builtin()), AsnClassification::None);
    }

    #[tokio::test]
    async fn from_file_nonexistent_returns_err() {
        let result = AsnPatterns::from_file("/nonexistent/asn_patterns.toml").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn from_file_asn48152_classifies_as_hosting() {
        let result = AsnPatterns::from_file("data/asn_patterns.toml").await;
        assert!(result.is_ok(), "TOML parse failed: {:?}", result.err());
        let patterns = result.unwrap();
        let class = classify_asn(Some(48152), None, &patterns);
        assert_eq!(
            class,
            AsnClassification::Hosting { provider: "Digital Realty Frankfurt GmbH".to_string() }
        );
    }
}
