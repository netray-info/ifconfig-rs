use ip_network_table::IpNetworkTable;
use std::net::IpAddr;

pub struct VpnRanges {
    table: IpNetworkTable<()>,
}

impl VpnRanges {
    pub async fn from_file(path: &str) -> Option<Self> {
        let contents = tokio::fs::read_to_string(path).await.ok()?;
        let mut table = IpNetworkTable::new();
        let mut count = 0u32;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Ok(network) = line.parse::<ip_network::IpNetwork>() {
                table.insert(network, ());
                count += 1;
            }
        }

        if count == 0 {
            return None;
        }

        Some(VpnRanges { table })
    }

    pub fn lookup(&self, ip: IpAddr) -> bool {
        self.table.longest_match(ip).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    async fn make_db(content: &str) -> VpnRanges {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_vpn_{}.txt", id));
        std::fs::write(&path, content).unwrap();
        let db = VpnRanges::from_file(path.to_str().unwrap()).await.unwrap();
        let _ = std::fs::remove_file(&path);
        db
    }

    #[tokio::test]
    async fn lookup_match() {
        let db = make_db("10.0.0.0/8\n192.168.0.0/16\n").await;
        assert!(db.lookup("10.1.2.3".parse().unwrap()));
        assert!(db.lookup("192.168.1.1".parse().unwrap()));
    }

    #[tokio::test]
    async fn lookup_miss() {
        let db = make_db("10.0.0.0/8\n").await;
        assert!(!db.lookup("172.16.0.1".parse().unwrap()));
    }

    #[tokio::test]
    async fn skips_comments() {
        let db = make_db("# comment\n10.0.0.0/8\n").await;
        assert!(db.lookup("10.1.2.3".parse().unwrap()));
    }

    #[tokio::test]
    async fn empty_file_returns_none() {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_vpn_empty_{}.txt", id));
        std::fs::write(&path, "").unwrap();
        assert!(VpnRanges::from_file(path.to_str().unwrap()).await.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn ipv6_lookup() {
        let db = make_db("2001:db8::/32\n").await;
        assert!(db.lookup("2001:db8::1".parse().unwrap()));
        assert!(!db.lookup("2001:db9::1".parse().unwrap()));
    }
}
