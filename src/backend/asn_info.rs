use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AsnCategory {
    Hosting,
    Isp,
    Business,
    EducationResearch,
    GovernmentAdmin,
    Unknown,
}

pub struct AsnMeta {
    pub category: AsnCategory,
    pub network_role: Option<String>,
    pub asn_registered: Option<String>,
}

pub struct AsnInfo {
    map: HashMap<u32, AsnMeta>,
}

#[derive(Deserialize)]
struct RawRecord {
    asn: u32,
    category: String,
    network_role: String,
    #[serde(default)]
    registered: String,
}

impl AsnInfo {
    pub async fn from_file(path: &str) -> Result<Self, String> {
        let contents = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("failed to read {path}: {e}"))?;
        let mut map = HashMap::new();
        for (i, line) in contents.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let r: RawRecord = serde_json::from_str(line).map_err(|e| format!("line {}: {e}", i + 1))?;
            let category = match r.category.as_str() {
                "hosting" => AsnCategory::Hosting,
                "isp" => AsnCategory::Isp,
                "business" => AsnCategory::Business,
                "education_research" => AsnCategory::EducationResearch,
                "government_admin" => AsnCategory::GovernmentAdmin,
                _ => AsnCategory::Unknown,
            };
            let network_role = if r.network_role.is_empty() {
                None
            } else {
                Some(r.network_role)
            };
            let asn_registered = if r.registered.is_empty() {
                None
            } else {
                Some(r.registered)
            };
            map.insert(
                r.asn,
                AsnMeta {
                    category,
                    network_role,
                    asn_registered,
                },
            );
        }
        Ok(AsnInfo { map })
    }

    pub fn lookup(&self, asn: u32) -> Option<&AsnMeta> {
        self.map.get(&asn)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    async fn load_from_str(content: &str) -> AsnInfo {
        let id = TEST_ID.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("asn_info_test_{}.jsonl", id));
        tokio::fs::write(&path, content).await.unwrap();
        let result = AsnInfo::from_file(path.to_str().unwrap()).await;
        let _ = tokio::fs::remove_file(&path).await;
        result.unwrap()
    }

    #[tokio::test]
    async fn lookup_hosting() {
        let content = r#"{"asn":13335,"category":"hosting","network_role":"content_network"}"#;
        let db = load_from_str(content).await;
        let meta = db.lookup(13335).unwrap();
        assert_eq!(meta.category, AsnCategory::Hosting);
        assert_eq!(meta.network_role.as_deref(), Some("content_network"));
    }

    #[tokio::test]
    async fn lookup_isp() {
        let content = r#"{"asn":7922,"category":"isp","network_role":"access_provider"}"#;
        let db = load_from_str(content).await;
        let meta = db.lookup(7922).unwrap();
        assert_eq!(meta.category, AsnCategory::Isp);
        assert_eq!(meta.network_role.as_deref(), Some("access_provider"));
    }

    #[tokio::test]
    async fn lookup_unknown_asn() {
        let content = r#"{"asn":13335,"category":"hosting","network_role":"content_network"}"#;
        let db = load_from_str(content).await;
        assert!(db.lookup(99999).is_none());
    }

    #[tokio::test]
    async fn from_file_nonexistent_returns_err() {
        let result = AsnInfo::from_file("/nonexistent/as_metadata.jsonl").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn empty_network_role_becomes_none() {
        let content = r#"{"asn":1234,"category":"isp","network_role":""}"#;
        let db = load_from_str(content).await;
        let meta = db.lookup(1234).unwrap();
        assert!(meta.network_role.is_none());
    }

    #[tokio::test]
    async fn registered_date_parsed() {
        let content = r#"{"asn":4711,"category":"business","network_role":"stub","registered":"1997-03-14"}"#;
        let db = load_from_str(content).await;
        assert_eq!(db.lookup(4711).unwrap().asn_registered.as_deref(), Some("1997-03-14"));
    }

    #[tokio::test]
    async fn empty_registered_becomes_none() {
        let content = r#"{"asn":1234,"category":"isp","network_role":"","registered":""}"#;
        let db = load_from_str(content).await;
        assert!(db.lookup(1234).unwrap().asn_registered.is_none());
    }
}
