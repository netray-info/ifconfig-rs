use std::collections::HashSet;
use std::net::IpAddr;

pub struct FeodoBotnetIps(Option<HashSet<IpAddr>>);

impl FeodoBotnetIps {
    pub async fn from_file(path: &str) -> Self {
        let set = tokio::fs::read_to_string(path)
            .await
            .ok()
            .map(|contents| {
                contents
                    .lines()
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .filter_map(|line| line.trim().parse().ok())
                    .collect::<HashSet<IpAddr>>()
            })
            .filter(|set| !set.is_empty());
        FeodoBotnetIps(set)
    }

    pub fn empty() -> Self {
        FeodoBotnetIps(None)
    }

    pub fn is_loaded(&self) -> bool {
        self.0.is_some()
    }

    pub fn lookup(&self, addr: &IpAddr) -> Option<bool> {
        self.0.as_ref().map(|set| set.contains(addr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_returns_none() {
        let nodes = FeodoBotnetIps::empty();
        let addr: IpAddr = "1.2.3.4".parse().unwrap();
        assert_eq!(nodes.lookup(&addr), None);
    }

    #[tokio::test]
    async fn from_file_missing_returns_none() {
        let nodes = FeodoBotnetIps::from_file("/nonexistent/path/feodo.txt").await;
        let addr: IpAddr = "1.2.3.4".parse().unwrap();
        assert_eq!(nodes.lookup(&addr), None);
    }

    #[test]
    fn lookup_found() {
        let mut set = HashSet::new();
        set.insert("198.51.100.1".parse::<IpAddr>().unwrap());
        let nodes = FeodoBotnetIps(Some(set));
        assert_eq!(nodes.lookup(&"198.51.100.1".parse().unwrap()), Some(true));
    }

    #[test]
    fn lookup_not_found() {
        let mut set = HashSet::new();
        set.insert("198.51.100.1".parse::<IpAddr>().unwrap());
        let nodes = FeodoBotnetIps(Some(set));
        assert_eq!(nodes.lookup(&"10.0.0.1".parse().unwrap()), Some(false));
    }

    #[test]
    fn skips_comment_lines() {
        let mut set = HashSet::new();
        // The Feodo blocklist has comment lines starting with #
        set.insert("198.51.100.1".parse::<IpAddr>().unwrap());
        let nodes = FeodoBotnetIps(Some(set));
        assert_eq!(nodes.lookup(&"198.51.100.1".parse().unwrap()), Some(true));
    }
}
