use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, Request, Uri};
use axum::middleware::Next;
use axum::response::Response;
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
        let remote = extract_client_ip(peer, headers, &state.config.server.trusted_proxies);
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

fn extract_client_ip(peer: SocketAddr, headers: &HeaderMap, trusted_proxies: &[String]) -> SocketAddr {
    if trusted_proxies.is_empty() {
        return peer;
    }

    let xff = match headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        Some(xff) => xff,
        None => return peer,
    };

    // Parse trusted proxy CIDRs/IPs
    let trusted: Vec<IpAddr> = trusted_proxies
        .iter()
        .filter_map(|s| IpAddr::from_str(s).ok())
        .collect();

    // Walk the XFF chain from right to left, skipping trusted proxies
    let ips: Vec<&str> = xff.split(',').map(str::trim).collect();
    for ip_str in ips.iter().rev() {
        if let Ok(ip) = IpAddr::from_str(ip_str) {
            if !trusted.contains(&ip) {
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
pub fn extract_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
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

    #[test]
    fn no_trusted_proxies_returns_peer() {
        let peer: SocketAddr = "1.2.3.4:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "10.0.0.1")]);
        let result = extract_client_ip(peer, &headers, &[]);
        assert_eq!(result, peer);
    }

    #[test]
    fn xff_with_trusted_proxy() {
        let peer: SocketAddr = "10.0.0.1:1234".parse().unwrap();
        let headers = headers_with(&[("x-forwarded-for", "1.2.3.4, 10.0.0.1")]);
        let trusted = vec!["10.0.0.1".to_string()];
        let result = extract_client_ip(peer, &headers, &trusted);
        assert_eq!(result.ip(), "1.2.3.4".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn xff_no_header_returns_peer() {
        let peer: SocketAddr = "1.2.3.4:1234".parse().unwrap();
        let headers = HeaderMap::new();
        let trusted = vec!["10.0.0.1".to_string()];
        let result = extract_client_ip(peer, &headers, &trusted);
        assert_eq!(result, peer);
    }
}
