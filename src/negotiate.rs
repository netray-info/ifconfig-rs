use axum::http::HeaderMap;
use regex::RegexSet;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiatedFormat {
    Plain,
    Json,
    Yaml,
    Toml,
    Csv,
    Html,
}

static KNOWN_CLI_CLIENTS: LazyLock<RegexSet> =
    LazyLock::new(|| RegexSet::new([r"curl", r"HTTPie", r"HTTP Library", r"Wget"]).unwrap());

pub fn is_cli_client(ua: Option<&str>, accept: Option<&str>) -> bool {
    matches!((ua, accept), (Some(ua), Some("*/*")) if KNOWN_CLI_CLIENTS.is_match(ua))
}

pub fn negotiate(suffix: Option<&str>, headers: &HeaderMap) -> NegotiatedFormat {
    // 1. Format suffix has highest priority
    if let Some(fmt) = suffix {
        return match fmt {
            "json" => NegotiatedFormat::Json,
            "yaml" => NegotiatedFormat::Yaml,
            "toml" => NegotiatedFormat::Toml,
            "csv" => NegotiatedFormat::Csv,
            _ => NegotiatedFormat::Html, // unknown suffix falls through
        };
    }

    let ua = headers.get("user-agent").and_then(|v| v.to_str().ok());
    let accept = headers.get("accept").and_then(|v| v.to_str().ok());

    // 2. CLI detection: known CLI + Accept: */*
    if is_cli_client(ua, accept) {
        return NegotiatedFormat::Plain;
    }

    // 3. Accept header matching
    if let Some(accept) = accept {
        // Check each media type in the Accept header
        for media in accept.split(',').map(|s| s.split(';').next().unwrap_or("").trim()) {
            match media {
                "application/json" => return NegotiatedFormat::Json,
                "application/yaml" => return NegotiatedFormat::Yaml,
                "application/toml" => return NegotiatedFormat::Toml,
                "text/csv" => return NegotiatedFormat::Csv,
                "text/plain" => return NegotiatedFormat::Plain,
                "text/html" => return NegotiatedFormat::Html,
                _ => {}
            }
        }
    }

    // 4. Default → HTML (SPA)
    NegotiatedFormat::Html
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                axum::http::header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_str(v).unwrap(),
            );
        }
        map
    }

    #[test]
    fn suffix_json() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(Some("json"), &h), NegotiatedFormat::Json);
    }

    #[test]
    fn suffix_yaml() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(Some("yaml"), &h), NegotiatedFormat::Yaml);
    }

    #[test]
    fn suffix_toml() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(Some("toml"), &h), NegotiatedFormat::Toml);
    }

    #[test]
    fn suffix_csv() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(Some("csv"), &h), NegotiatedFormat::Csv);
    }

    #[test]
    fn cli_curl() {
        let h = headers_with(&[("user-agent", "curl/7.54.0"), ("accept", "*/*")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Plain);
    }

    #[test]
    fn cli_httpie() {
        let h = headers_with(&[("user-agent", "HTTPie/0.9.9"), ("accept", "*/*")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Plain);
    }

    #[test]
    fn cli_wget() {
        let h = headers_with(&[("user-agent", "Wget/1.19.5 (darwin17.5.0)"), ("accept", "*/*")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Plain);
    }

    #[test]
    fn accept_json() {
        let h = headers_with(&[("accept", "application/json")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Json);
    }

    #[test]
    fn accept_yaml() {
        let h = headers_with(&[("accept", "application/yaml")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Yaml);
    }

    #[test]
    fn accept_toml() {
        let h = headers_with(&[("accept", "application/toml")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Toml);
    }

    #[test]
    fn accept_csv() {
        let h = headers_with(&[("accept", "text/csv")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Csv);
    }

    #[test]
    fn accept_plain() {
        let h = headers_with(&[("accept", "text/plain")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Plain);
    }

    #[test]
    fn accept_html() {
        let h = headers_with(&[("accept", "text/html")]);
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Html);
    }

    #[test]
    fn default_html() {
        let h = HeaderMap::new();
        assert_eq!(negotiate(None, &h), NegotiatedFormat::Html);
    }

    #[test]
    fn suffix_overrides_accept() {
        let h = headers_with(&[("accept", "text/html")]);
        assert_eq!(negotiate(Some("json"), &h), NegotiatedFormat::Json);
    }

    #[test]
    fn is_cli_client_true() {
        assert!(is_cli_client(Some("curl/7.54.0"), Some("*/*")));
        assert!(is_cli_client(Some("HTTPie/0.9.9"), Some("*/*")));
        assert!(is_cli_client(Some("Wget/1.19.5"), Some("*/*")));
    }

    #[test]
    fn is_cli_client_false() {
        assert!(!is_cli_client(Some("Mozilla/5.0"), Some("*/*")));
        assert!(!is_cli_client(Some("curl/7.54.0"), Some("text/html")));
        assert!(!is_cli_client(None, Some("*/*")));
        assert!(!is_cli_client(Some("curl/7.54.0"), None));
    }
}
