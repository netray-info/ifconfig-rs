use ip_network_table::IpNetworkTable;
use std::net::IpAddr;

pub struct DatacenterRanges {
    table: IpNetworkTable<()>,
}

impl DatacenterRanges {
    pub fn from_file(path: &str) -> Option<Self> {
        let contents = std::fs::read_to_string(path).ok()?;
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

        Some(DatacenterRanges { table })
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

    fn make_db(content: &str) -> DatacenterRanges {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_dc_{}.txt", id));
        std::fs::write(&path, content).unwrap();
        let db = DatacenterRanges::from_file(path.to_str().unwrap()).unwrap();
        let _ = std::fs::remove_file(&path);
        db
    }

    #[test]
    fn lookup_match() {
        let db = make_db("10.0.0.0/8\n192.168.0.0/16\n");
        assert!(db.lookup("10.1.2.3".parse().unwrap()));
        assert!(db.lookup("192.168.1.1".parse().unwrap()));
    }

    #[test]
    fn lookup_miss() {
        let db = make_db("10.0.0.0/8\n");
        assert!(!db.lookup("172.16.0.1".parse().unwrap()));
    }

    #[test]
    fn skips_comments() {
        let db = make_db("# comment\n10.0.0.0/8\n");
        assert!(db.lookup("10.1.2.3".parse().unwrap()));
    }

    #[test]
    fn empty_file_returns_none() {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_dc_empty_{}.txt", id));
        std::fs::write(&path, "").unwrap();
        assert!(DatacenterRanges::from_file(path.to_str().unwrap()).is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn ipv6_lookup() {
        let db = make_db("2001:db8::/32\n");
        assert!(db.lookup("2001:db8::1".parse().unwrap()));
        assert!(!db.lookup("2001:db9::1".parse().unwrap()));
    }
}
