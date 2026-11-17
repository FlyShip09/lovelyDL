//! The client for addresses that come from downloaded pages (thumbnails). Such an address may
//! only reach the public internet: never this PC or the local network, whether it names an IP,
//! a host name that resolves to one, or redirects there.

use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use reqwest::redirect;

pub fn is_public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let [a, b, ..] = v4.octets();
            let shared = a == 100 && (b & 0xc0) == 64; // 100.64.0.0/10, carrier-grade NAT
            !(v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                || shared)
        }
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => is_public(IpAddr::V4(v4)),
            None => !(v6.is_loopback() || v6.is_unspecified() || v6.is_unique_local() || v6.is_unicast_link_local()),
        },
    }
}

/// Whether a URL's host is acceptable before any request: IP literals skip DNS, so they are
/// checked here; names are checked by the resolver.
pub fn public_host(url: &url::Url) -> bool {
    match url.host() {
        Some(url::Host::Ipv4(ip)) => is_public(IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => is_public(IpAddr::V6(ip)),
        Some(url::Host::Domain(_)) => true,
        None => false,
    }
}

struct PublicOnly;

impl Resolve for PublicOnly {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let found: Vec<SocketAddr> =
                tokio::net::lookup_host((host.as_str(), 0)).await?.filter(|a| is_public(a.ip())).collect();
            if found.is_empty() {
                return Err("that address is not on the public internet".into());
            }
            Ok(Box::new(found.into_iter()) as Addrs)
        })
    }
}

pub fn public_client(user_agent: &str) -> reqwest::Client {
    let redirects = redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 5 {
            attempt.error("too many redirects")
        } else if matches!(attempt.url().scheme(), "http" | "https") && public_host(attempt.url()) {
            attempt.follow()
        } else {
            attempt.stop()
        }
    });
    reqwest::Client::builder()
        .user_agent(user_agent)
        .connect_timeout(Duration::from_secs(15))
        .read_timeout(Duration::from_secs(30))
        .redirect(redirects)
        .dns_resolver(PublicOnly)
        .build()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_public_addresses() {
        for private in
            ["127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.1.1", "169.254.169.254", "100.64.0.1", "0.0.0.0"]
        {
            assert!(!is_public(private.parse().unwrap()), "{private}");
        }
        for private in ["::1", "fe80::1", "fd00::1", "::ffff:192.168.1.1"] {
            assert!(!is_public(private.parse().unwrap()), "{private}");
        }
        for public in ["142.250.80.46", "2606:4700::1111"] {
            assert!(is_public(public.parse().unwrap()), "{public}");
        }
        assert!(!public_host(&url::Url::parse("http://192.168.0.1/x.jpg").unwrap()));
        assert!(public_host(&url::Url::parse("https://i.ytimg.com/vi/x/hq.jpg").unwrap()));
    }

    #[tokio::test]
    async fn names_that_resolve_to_this_pc_are_refused() {
        let client = public_client("test");
        let error = client.get("http://localhost:9/thumb.jpg").send().await.unwrap_err();
        assert!(format!("{error:?}").contains("public internet"), "{error:?}");
    }
}
