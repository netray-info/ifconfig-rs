use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, Request, Uri};
use axum::middleware::Next;
use axum::response::Response;
use ip_network::IpNetwork;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct RequesterInfo {
    pub remote: SocketAddr,
    pub user_agent: Option<String>,
    pub uri: String,
}

impl RequesterInfo {
    pub fn from_request(
        connect_info: &ConnectInfo<SocketAddr>,
        headers: &HeaderMap,
        uri: &Uri,
        state: &AppState,
    ) -> Self {
        let peer = connect_info.0;
        let remote = extract_client_ip(peer, headers, &state.trusted_proxies);
        let user_agent = headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let uri_str = uri.to_string();

        RequesterInfo {
            remote,
            user_agent,
            uri: uri_str,
        }
    }
}

fn extract_client_ip(peer: SocketAddr, headers: &HeaderMap, trusted_proxies: &[IpNetwork]) -> SocketAddr {
    if trusted_proxies.is_empty() {
        return peer;
    }

    // Only trust proxy headers if the direct peer is itself a trusted proxy
    let peer_trusted = trusted_proxies.iter().any(|net| net.contains(peer.ip()));
    if !peer_trusted {
        return peer;
    }

    // Priority 1: CF-Connecting-IP (set exclusively by Cloudflare; single IP, no chain)
    if let Some(ip_str) = headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = IpAddr::from_str(ip_str.trim()) {
            return SocketAddr::new(ip, peer.port());
        }
    }

    // Priority 2: X-Real-IP (set by nginx/similar; single IP, no chain)
    if let Some(ip_str) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = IpAddr::from_str(ip_str.trim()) {
            return SocketAddr::new(ip, peer.port());
        }
    }

    // Priority 3: X-Forwarded-For chain (right-to-left walk, skip trusted proxies)
    let xff = match headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        Some(xff) => xff,
        None => return peer,
    };

    let is_trusted = |ip: IpAddr| trusted_proxies.iter().any(|net| net.contains(ip));

    // Walk the XFF chain from right to left, skipping trusted proxies
    let ips: Vec<&str> = xff.split(',').map(str::trim).collect();
    for ip_str in ips.iter().rev() {
        if let Ok(ip) = IpAddr::from_str(ip_str) {
            if !is_trusted(ip) {
                return SocketAddr::new(ip, peer.port());
            }
        }
    }

    // If all are trusted, use the leftmost
    if let Some(ip_str) = ips.first() {
        if let Ok(ip) = IpAddr::from_str(ip_str.trim()) {
            return SocketAddr::new(ip, peer.port());
        }
    }

    peer
}

/// Extract request headers as a simple Vec of (name, value) pairs.
/// Capped at 64 headers; values longer than 1 KB are truncated.
pub fn extract_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    const MAX_HEADERS: usize = 64;
    const MAX_VALUE_LEN: usize = 1024;
    headers
        .iter()
        .take(MAX_HEADERS)
        .map(|(name, value)| {
            let v = value.to_str().unwrap_or("");
            // Header values are ASCII; safe to slice at byte boundary.
            let v = if v.len() > MAX_VALUE_LEN {
                &v[..MAX_VALUE_LEN]
            } else {
                v
            };
            (name.to_string(), v.to_string())
        })
        .collect()
}

/// Remove headers whose names match any of the provided regex filters.
pub fn filter_headers(headers: Vec<(String, String)>, filters: &regex::RegexSet) -> Vec<(String, String)> {
    if filters.is_empty() {
        return headers;
    }
    headers
        .into_iter()
        .filter(|(name, _)| !filters.is_match(name))
        .collect()
}

/// Middleware that extracts RequesterInfo and stores it as a request extension.
pub async fn requester_info_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let info = RequesterInfo::from_request(&ConnectInfo(addr), req.headers(), req.uri(), &state);
    tracing::Span::current().record("client_ip", tracing::field::display(info.remote.ip()));
    req.extensions_mut().insert(info);
    next.run(req).await
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

    fn net(s: &str) -> IpNetwork {
        s.parse().unwrap()
    }

    #[test]
    fn no_trusted_proxies_returns_peer() {
        let peer: SocketAddr = "1.2.3.4:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "10.0.0.1")]);
        let result = extract_client_ip(peer, &headers, &[]);
        assert_eq!(result, peer);
    }

    #[test]
    fn xff_with_trusted_proxy_exact_ip() {
        let peer: SocketAddr = "10.0.0.1:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "1.2.3.4, 10.0.0.1")]);
        let trusted = vec![net("10.0.0.1/32")];
        let result = extract_client_ip(peer, &headers, &trusted);
        assert_eq!(result.ip(), "1.2.3.4".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn xff_with_trusted_proxy_cidr() {
        let peer: SocketAddr = "10.0.0.5:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "1.2.3.4, 10.0.0.5")]);
        let trusted = vec![net("10.0.0.0/8")];
        let result = extract_client_ip(peer, &headers, &trusted);
        assert_eq!(result.ip(), "1.2.3.4".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn xff_with_multiple_trusted_cidrs() {
        let peer: SocketAddr = "172.16.0.1:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "8.8.8.8, 10.0.0.1, 172.16.0.1")]);
        let trusted = vec![net("10.0.0.0/8"), net("172.16.0.0/12")];
        let result = extract_client_ip(peer, &headers, &trusted);
        assert_eq!(result.ip(), "8.8.8.8".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn xff_untrusted_peer_ignores_xff() {
        let peer: SocketAddr = "192.168.1.1:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "1.2.3.4, 192.168.1.1")]);
        let trusted = vec![net("10.0.0.0/8")];
        let result = extract_client_ip(peer, &headers, &trusted);
        // Peer 192.168.1.1 is not in trusted proxies, so XFF is ignored
        assert_eq!(result, peer);
    }

    #[test]
    fn xff_spoofed_by_untrusted_peer_returns_peer() {
        let peer: SocketAddr = "203.0.113.1:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "10.0.0.1")]);
        let trusted = vec![net("10.0.0.0/8")];
        let result = extract_client_ip(peer, &headers, &trusted);
        // Peer is not trusted, so XFF header is not trusted regardless of content
        assert_eq!(result, peer);
    }

    #[test]
    fn filter_headers_empty_filters() {
        let headers = vec![
            ("host".into(), "example.com".into()),
            ("x-koyeb-id".into(), "abc".into()),
        ];
        let filters = regex::RegexSet::empty();
        let result = filter_headers(headers.clone(), &filters);
        assert_eq!(result, headers);
    }

    #[test]
    fn filter_headers_removes_matching() {
        let headers = vec![
            ("host".into(), "example.com".into()),
            ("x-koyeb-id".into(), "abc".into()),
            ("x-koyeb-region".into(), "par".into()),
            ("cf-ray".into(), "123".into()),
            ("accept".into(), "*/*".into()),
        ];
        let filters = regex::RegexSet::new(["^x-koyeb-", "^cf-"]).unwrap();
        let result = filter_headers(headers, &filters);
        assert_eq!(
            result,
            vec![("host".into(), "example.com".into()), ("accept".into(), "*/*".into()),]
        );
    }

    #[test]
    fn filter_headers_no_match_keeps_all() {
        let headers = vec![
            ("host".into(), "example.com".into()),
            ("accept".into(), "text/html".into()),
        ];
        let filters = regex::RegexSet::new(["^x-koyeb-"]).unwrap();
        let result = filter_headers(headers.clone(), &filters);
        assert_eq!(result, headers);
    }

    #[test]
    fn xff_no_header_returns_peer() {
        let peer: SocketAddr = "1.2.3.4:1234".parse().unwrap();
        let headers = HeaderMap::new();
        let trusted = vec![net("10.0.0.1/32")];
        let result = extract_client_ip(peer, &headers, &trusted);
        assert_eq!(result, peer);
    }
}
