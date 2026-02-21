use ip_network_table::IpNetworkTable;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BotInfo {
    pub provider: String,
}

#[derive(Deserialize)]
struct JsonlEntry {
    cidr: String,
    provider: String,
}

pub struct BotDb {
    table: IpNetworkTable<BotInfo>,
}

impl BotDb {
    pub async fn from_file(path: &str) -> Option<Self> {
        let contents = tokio::fs::read_to_string(path).await.ok()?;
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
                        BotInfo {
                            provider: entry.provider,
                        },
                    );
                    count += 1;
                }
            }
        }

        if count == 0 {
            return None;
        }

        Some(BotDb { table })
    }

    pub fn lookup(&self, ip: IpAddr) -> Option<&BotInfo> {
        self.table.longest_match(ip).map(|(_net, info)| info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    async fn make_db(jsonl: &str) -> BotDb {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_bot_{}.jsonl", id));
        std::fs::write(&path, jsonl).unwrap();
        let db = BotDb::from_file(path.to_str().unwrap()).await.unwrap();
        let _ = std::fs::remove_file(&path);
        db
    }

    #[tokio::test]
    async fn lookup_googlebot() {
        let jsonl = r#"{"cidr":"66.249.64.0/19","provider":"googlebot"}"#;
        let db = make_db(jsonl).await;
        let result = db.lookup("66.249.64.1".parse().unwrap());
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, "googlebot");
    }

    #[tokio::test]
    async fn lookup_miss() {
        let jsonl = r#"{"cidr":"66.249.64.0/19","provider":"googlebot"}"#;
        let db = make_db(jsonl).await;
        assert!(db.lookup("192.168.1.1".parse().unwrap()).is_none());
    }

    #[tokio::test]
    async fn empty_file_returns_none() {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_bot_empty_{}.jsonl", id));
        std::fs::write(&path, "").unwrap();
        assert!(BotDb::from_file(path.to_str().unwrap()).await.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn ipv6_lookup() {
        let jsonl = r#"{"cidr":"2001:4860:4801::/48","provider":"googlebot"}"#;
        let db = make_db(jsonl).await;
        let result = db.lookup("2001:4860:4801::1".parse().unwrap());
        assert!(result.is_some());
        assert_eq!(result.unwrap().provider, "googlebot");
    }

    #[tokio::test]
    async fn multiple_providers() {
        let jsonl = concat!(
            r#"{"cidr":"66.249.64.0/19","provider":"googlebot"}"#,
            "\n",
            r#"{"cidr":"40.77.167.0/24","provider":"bingbot"}"#,
        );
        let db = make_db(jsonl).await;
        assert_eq!(db.lookup("66.249.64.1".parse().unwrap()).unwrap().provider, "googlebot");
        assert_eq!(db.lookup("40.77.167.1".parse().unwrap()).unwrap().provider, "bingbot");
    }
}
