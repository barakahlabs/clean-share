//! Keeps requests on the public internet: http(s) only, and no private, local or reserved
//! addresses, whether written in the link or returned by DNS.

use std::{
  net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr},
  sync::Arc,
};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use url::{Host, Url};

use crate::{Error, Result};

/// Parses `url` and checks its scheme, and its host if that's an IP address (DNS names are
/// checked when they resolve, by [`PublicResolver`]).
pub fn check(url: &str, allow_private: bool) -> Result<Url> {
  let parsed = Url::parse(url).map_err(|_| Error::InvalidUrl(url.to_string()))?;
  check_url(&parsed, allow_private)?;
  Ok(parsed)
}

pub fn check_url(url: &Url, allow_private: bool) -> Result<()> {
  if !matches!(url.scheme(), "http" | "https") {
    return Err(Error::InvalidUrl(url.to_string()));
  }
  let ip = match url.host() {
    Some(Host::Ipv4(ip)) => IpAddr::V4(ip),
    Some(Host::Ipv6(ip)) => IpAddr::V6(ip),
    Some(Host::Domain(name)) if name.eq_ignore_ascii_case("localhost") || name.ends_with(".local") => {
      return if allow_private { Ok(()) } else { Err(Error::Blocked(name.to_string())) };
    }
    Some(Host::Domain(_)) => return Ok(()),
    None => return Err(Error::InvalidUrl(url.to_string())),
  };
  if allow_private || is_public(ip) { Ok(()) } else { Err(Error::Blocked(ip.to_string())) }
}

/// Resolves with the system resolver and drops non-public addresses; fails if none are left.
pub struct PublicResolver;

impl Resolve for PublicResolver {
  fn resolve(&self, name: Name) -> Resolving {
    Box::pin(async move {
      let host = name.as_str().to_string();
      let addrs: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0))
        .await?
        .filter(|addr| is_public(addr.ip()))
        .collect();
      if addrs.is_empty() {
        return Err(Box::new(Error::Blocked(host)) as Box<dyn std::error::Error + Send + Sync>);
      }
      Ok(Box::new(addrs.into_iter()) as Addrs)
    })
  }
}

pub fn resolver() -> Arc<PublicResolver> {
  Arc::new(PublicResolver)
}

pub fn is_public(ip: IpAddr) -> bool {
  match ip {
    IpAddr::V4(ip) => is_public_v4(ip),
    IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
      Some(v4) => is_public_v4(v4),
      None => is_public_v6(ip),
    },
  }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
  let [a, b, ..] = ip.octets();
  !(ip.is_private()
    || ip.is_loopback()
    || ip.is_link_local()
    || ip.is_unspecified()
    || ip.is_broadcast()
    || ip.is_multicast()
    || ip.is_documentation()
    || a == 0
    // Shared address space (carrier-grade NAT), 100.64.0.0/10.
    || (a == 100 && (64..128).contains(&b))
    // Reserved for future use, 240.0.0.0/4.
    || a >= 240)
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
  let first = ip.segments()[0];
  !(ip.is_loopback()
    || ip.is_unspecified()
    || ip.is_multicast()
    // Unique local, fc00::/7.
    || (first & 0xfe00) == 0xfc00
    // Link-local, fe80::/10.
    || (first & 0xffc0) == 0xfe80
    // Documentation, 2001:db8::/32.
    || (first == 0x2001 && ip.segments()[1] == 0x0db8))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn public(ip: &str) -> bool {
    is_public(ip.parse().unwrap())
  }

  #[test]
  fn private_and_local_addresses_are_not_public() {
    for ip in [
      "127.0.0.1", "10.1.2.3", "172.16.0.1", "192.168.1.1", "169.254.1.1", "100.64.0.1", "0.0.0.0",
      "255.255.255.255", "::1", "fd00::1", "fe80::1", "::ffff:192.168.0.1", "2001:db8::1",
    ] {
      assert!(!public(ip), "{ip}");
    }
    for ip in ["1.1.1.1", "142.250.185.78", "100.128.0.1", "2a00:1450:4001::200e"] {
      assert!(public(ip), "{ip}");
    }
  }

  #[test]
  fn only_web_links_with_public_ip_hosts_pass() {
    assert!(check("https://example.com/a", false).is_ok());
    assert!(matches!(check("http://192.168.1.1/admin", false), Err(Error::Blocked(_))));
    assert!(matches!(check("http://[::1]:8080/", false), Err(Error::Blocked(_))));
    assert!(matches!(check("http://localhost:3000/", false), Err(Error::Blocked(_))));
    assert!(matches!(check("http://printer.local/", false), Err(Error::Blocked(_))));
    assert!(matches!(check("file:///etc/passwd", false), Err(Error::InvalidUrl(_))));
    assert!(matches!(check("not a link", false), Err(Error::InvalidUrl(_))));
    assert!(check("http://127.0.0.1:8080/", true).is_ok());
  }
}
