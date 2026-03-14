use ip_network_table::IpNetworkTable;
use std::net::IpAddr;

pub struct SpamhausDrop {
    table: IpNetworkTable<()>,
}

impl SpamhausDrop {
    pub async fn from_file(path: &str) -> Option<Self> {
        let contents = tokio::fs::read_to_string(path).await.ok()?;
        let mut table = IpNetworkTable::new();
        let mut count = 0u32;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
                continue;
            }
            let line = line.split(" ;").next().unwrap_or(line);
            if let Ok(network) = line.parse::<ip_network::IpNetwork>() {
                table.insert(network, ());
                count += 1;
            }
        }

        if count == 0 {
            return None;
        }

        Some(SpamhausDrop { table })
    }

    pub fn lookup(&self, ip: IpAddr) -> bool {
        self.table.longest_match(ip).is_some()
    }

    pub fn len(&self) -> usize {
        let (v4, v6) = self.table.len();
        v4 + v6
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    async fn make_db(content: &str) -> SpamhausDrop {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_spam_{}.txt", id));
        std::fs::write(&path, content).unwrap();
        let db = SpamhausDrop::from_file(path.to_str().unwrap()).await.unwrap();
        let _ = std::fs::remove_file(&path);
        db
    }

    #[tokio::test]
    async fn lookup_match() {
        let db = make_db("1.10.16.0/20\n").await;
        assert!(db.lookup("1.10.16.1".parse().unwrap()));
    }

    #[tokio::test]
    async fn lookup_miss() {
        let db = make_db("1.10.16.0/20\n").await;
        assert!(!db.lookup("8.8.8.8".parse().unwrap()));
    }

    #[tokio::test]
    async fn skips_comments_and_semicolons() {
        let db = make_db("; Spamhaus DROP\n# comment\n1.10.16.0/20\n").await;
        assert!(db.lookup("1.10.16.1".parse().unwrap()));
    }

    #[tokio::test]
    async fn parses_inline_comments() {
        let db = make_db("1.10.16.0/20 ; SB123456\n2.56.0.0/14 ; SB654321\n").await;
        assert!(db.lookup("1.10.16.1".parse().unwrap()));
        assert!(db.lookup("2.56.0.1".parse().unwrap()));
    }

    #[tokio::test]
    async fn empty_file_returns_none() {
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("ifconfig_test_spam_empty_{}.txt", id));
        std::fs::write(&path, "").unwrap();
        assert!(SpamhausDrop::from_file(path.to_str().unwrap()).await.is_none());
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn ipv6_lookup() {
        let db = make_db("2001:db8::/32\n").await;
        assert!(db.lookup("2001:db8::1".parse().unwrap()));
        assert!(!db.lookup("2001:db9::1".parse().unwrap()));
    }
}
