use std::{collections::HashSet, net::IpAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, header::HeaderName},
    middleware::Next,
    response::Response,
};

const REAL_IP_HEADER: HeaderName = HeaderName::from_static("x-real-ip");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientIp(pub IpAddr);

#[derive(Debug, Clone)]
pub struct TrustedProxies {
    addresses: Arc<HashSet<IpAddr>>,
}

impl TrustedProxies {
    pub fn new(addresses: impl IntoIterator<Item = IpAddr>) -> Self {
        Self {
            addresses: Arc::new(addresses.into_iter().collect()),
        }
    }

    fn contains(&self, address: &IpAddr) -> bool {
        self.addresses.contains(address)
    }
}

pub async fn resolve_client_ip(
    State(trusted_proxies): State<TrustedProxies>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    mut request: Request,
    next: Next,
) -> Response {
    let client_ip = resolve(peer.ip(), request.headers(), &trusted_proxies);
    request.extensions_mut().insert(client_ip);
    next.run(request).await
}

fn resolve(peer_ip: IpAddr, headers: &HeaderMap, trusted_proxies: &TrustedProxies) -> ClientIp {
    if !trusted_proxies.contains(&peer_ip) {
        return ClientIp(peer_ip);
    }

    let mut values = headers.get_all(&REAL_IP_HEADER).iter();
    let Some(value) = values.next() else {
        return ClientIp(peer_ip);
    };
    if values.next().is_some() {
        return ClientIp(peer_ip);
    }

    let forwarded_ip = value
        .to_str()
        .ok()
        .and_then(|value| value.parse::<IpAddr>().ok());

    ClientIp(forwarded_ip.unwrap_or(peer_ip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn ip(value: &str) -> IpAddr {
        value.parse().expect("valid test IP")
    }

    #[test]
    fn direct_peer_is_preserved() {
        let headers = HeaderMap::new();
        let trusted = TrustedProxies::new([]);

        assert_eq!(
            resolve(ip("192.0.2.10"), &headers, &trusted),
            ClientIp(ip("192.0.2.10"))
        );
    }

    #[test]
    fn untrusted_peer_cannot_spoof_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(REAL_IP_HEADER, HeaderValue::from_static("198.51.100.25"));
        let trusted = TrustedProxies::new([ip("192.0.2.20")]);

        assert_eq!(
            resolve(ip("192.0.2.10"), &headers, &trusted),
            ClientIp(ip("192.0.2.10"))
        );
    }

    #[test]
    fn trusted_proxy_can_supply_one_valid_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert(REAL_IP_HEADER, HeaderValue::from_static("198.51.100.25"));
        let trusted = TrustedProxies::new([ip("192.0.2.10")]);

        assert_eq!(
            resolve(ip("192.0.2.10"), &headers, &trusted),
            ClientIp(ip("198.51.100.25"))
        );
    }

    #[test]
    fn forwarded_for_is_ignored() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("198.51.100.25"));
        let trusted = TrustedProxies::new([ip("192.0.2.10")]);

        assert_eq!(
            resolve(ip("192.0.2.10"), &headers, &trusted),
            ClientIp(ip("192.0.2.10"))
        );
    }

    #[test]
    fn invalid_or_ambiguous_real_ip_falls_back_to_peer() {
        let trusted = TrustedProxies::new([ip("192.0.2.10")]);
        for value in ["invalid", "198.51.100.25, 203.0.113.8", " 198.51.100.25"] {
            let mut headers = HeaderMap::new();
            headers.insert(
                REAL_IP_HEADER,
                HeaderValue::from_str(value).expect("header value"),
            );
            assert_eq!(
                resolve(ip("192.0.2.10"), &headers, &trusted),
                ClientIp(ip("192.0.2.10"))
            );
        }

        let mut duplicated = HeaderMap::new();
        duplicated.append(REAL_IP_HEADER, HeaderValue::from_static("198.51.100.25"));
        duplicated.append(REAL_IP_HEADER, HeaderValue::from_static("203.0.113.8"));
        assert_eq!(
            resolve(ip("192.0.2.10"), &duplicated, &trusted),
            ClientIp(ip("192.0.2.10"))
        );
    }
}
