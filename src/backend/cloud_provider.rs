use ip_network_table::IpNetworkTable;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct CloudProvider {
    pub provider: String,
    pub service: Option<String>,
    pub region: Option<String>,
}

#[derive(Deserialize)]
struct JsonlEntry {
    cidr: String,
    provider: String,
    service: Option<String>,
    region: Option<String>,
}

pub struct CloudProviderDb {
    table: IpNetworkTable<CloudProvider>,
}

impl CloudProviderDb {
    pub fn from_file(path: &str) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
        let mut table = IpNetworkTable::new();
        let mut count = 0u32;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<JsonlEntry>(line) {
                if let Ok(network) = entry.cidr.parse::<ip_network::IpNetwork>() {
                    table.insert(
                        network,
                        CloudProvider {
                            provider: entry.provider,
                            service: entry.service,
                            region: entry.region,
                        },
                    );
                    count += 1;
                }
            }
        }

        if count == 0 {
            return None;
        }

        Some(CloudProviderDb { table })
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<&CloudProvider> {
        self.table.longest_match(ip).map(|(_net, provider)| provider)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn make_db(jsonl: &str) -> CloudProviderDb {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_cloud_{}.jsonl", id));
        std::fs::write(&path, jsonl).unwrap();
        let db = CloudProviderDb::from_file(path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        db
    }

    #[test]
    fn lookup_aws_ip() {
        let jsonl = r#"{"cidr":"3.2.34.0/26","provider":"aws","service":"EC2","region":"af-south-1"}"#;
        let db = make_db(jsonl);
        let result = db.lookup("3.2.34.1".parse().unwrap());
        assert!(result.is_some());
        let cp = result.unwrap();
        assert_eq!(cp.provider, "aws");
        assert_eq!(cp.service.as_deref(), Some("EC2"));
        assert_eq!(cp.region.as_deref(), Some("af-south-1"));
    }

    #[test]
    fn lookup_cloudflare_no_service() {
        let jsonl = r#"{"cidr":"104.16.0.0/13","provider":"cloudflare","service":null,"region":null}"#;
        let db = make_db(jsonl);
        let result = db.lookup("104.17.0.1".parse().unwrap());
        assert!(result.is_some());
        let cp = result.unwrap();
        assert_eq!(cp.provider, "cloudflare");
        assert_eq!(cp.service, None);
    }

    #[test]
    fn lookup_miss() {
        let jsonl = r#"{"cidr":"3.2.34.0/26","provider":"aws","service":"EC2","region":"af-south-1"}"#;
        let db = make_db(jsonl);
        assert!(db.lookup("192.168.1.1".parse().unwrap()).is_none());
    }

    #[test]
    fn longest_prefix_match() {
        let jsonl = concat!(
            r#"{"cidr":"3.0.0.0/8","provider":"aws","service":"AMAZON","region":"GLOBAL"}"#,
            "\n",
            r#"{"cidr":"3.2.34.0/26","provider":"aws","service":"EC2","region":"af-south-1"}"#,
        );
        let db = make_db(jsonl);
        // Specific match should win
        let result = db.lookup("3.2.34.1".parse().unwrap()).unwrap();
        assert_eq!(result.service.as_deref(), Some("EC2"));
        assert_eq!(result.region.as_deref(), Some("af-south-1"));
        // Broader match for other IPs in /8
        let result = db.lookup("3.5.0.1".parse().unwrap()).unwrap();
        assert_eq!(result.service.as_deref(), Some("AMAZON"));
    }

    #[test]
    fn empty_file_returns_none() {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_cloud_empty_{}.jsonl", id));
        std::fs::write(&path, "").unwrap();
        assert!(CloudProviderDb::from_file(path.to_str().unwrap()).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ipv6_lookup() {
        let jsonl = r#"{"cidr":"2600:1f00::/24","provider":"aws","service":"EC2","region":"us-east-1"}"#;
        let db = make_db(jsonl);
        let result = db.lookup("2600:1f00::1".parse().unwrap());
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, "aws");
    }
}
