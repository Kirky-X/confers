// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! The single source of truth for SSRF blocked IP networks.
//!
//! This module is included twice by design:
//!
//! - `crate::remote::poll` includes it directly (`remote` feature) to gate
//!   DNS resolution and every reqwest connection of the polled HTTP source.
//! - `crate::security::rules::ssrf` includes it via `#[path]`
//!   (`security-rules` feature) so the `SsrfValidator` reports the exact same
//!   blocked ranges the runtime enforcement uses.
//!
//! Keeping one list (instead of two hand-maintained copies) is the fix for
//! the audit finding the poll-side list previously lacked
//! `0.0.0.0/8`, so `https://0.0.0.0` slipped past the pinned resolver.
//!
//! Requires the `ipnet` dependency, which both the `remote` and the
//! `security-rules` features enable.

use ipnet::IpNet;
use std::net::IpAddr;
use std::sync::LazyLock;

/// Private/reserved IP ranges blocked for SSRF protection (single source).
///
/// IPv4:
/// - loopback (`127.0.0.0/8`), class A/B/C private, link-local, "this network"
///   (`0.0.0.0/8`), CGN shared space, IETF protocol assignments, the three
///   TEST-NET documentation ranges, multicast, reserved (`240.0.0.0/4`) and
///   the limited broadcast address.
///
/// IPv6:
/// - loopback, the unspecified address (`::/128`), unique-local, link-local
///   and the NAT64 well-known prefix (`64:ff9b::/96`), which proxies
///   IPv4-only networks and must never be reachable from a config fetch.
static BLOCKED_NETWORKS: LazyLock<Vec<IpNet>> = LazyLock::new(|| {
    vec![
        // --- IPv4 ---
        "0.0.0.0/8".parse().unwrap(),          // "this network"
        "127.0.0.0/8".parse().unwrap(),        // Loopback
        "10.0.0.0/8".parse().unwrap(),         // Class A private
        "172.16.0.0/12".parse().unwrap(),      // Class B private
        "192.168.0.0/16".parse().unwrap(),     // Class C private
        "169.254.0.0/16".parse().unwrap(),     // Link-local (cloud metadata lives here)
        "100.64.0.0/10".parse().unwrap(),      // Carrier-grade NAT shared address space
        "192.0.0.0/24".parse().unwrap(),       // IETF protocol assignments
        "192.0.2.0/24".parse().unwrap(),       // TEST-NET-1
        "198.51.100.0/24".parse().unwrap(),    // TEST-NET-2
        "203.0.113.0/24".parse().unwrap(),     // TEST-NET-3
        "224.0.0.0/4".parse().unwrap(),        // Multicast
        "240.0.0.0/4".parse().unwrap(),        // Reserved (former class E)
        "255.255.255.255/32".parse().unwrap(), // Limited broadcast
        // --- IPv6 ---
        "::1/128".parse().unwrap(),      // Loopback
        "::/128".parse().unwrap(),       // Unspecified address
        "fc00::/7".parse().unwrap(),     // Unique local
        "fe80::/10".parse().unwrap(),    // Link-local
        "64:ff9b::/96".parse().unwrap(), // NAT64 well-known prefix (RFC 6052)
    ]
});

/// Check whether an IP address is in a blocked (private/reserved) range.
///
/// This is the shared decision function used by:
/// - the remote polled source (DNS pre-check + pinned reqwest resolver), and
/// - the `SsrfValidator` security rule.
///
/// IPv4-mapped IPv6 addresses (`::ffff:0:0/96`) are blocked outright: a
/// dual-stack socket can reach the embedded IPv4 target through them, and the
/// conservative rule also covers every mapped private/loopback range without
/// enumerating them.
pub fn is_ip_blocked(ip: IpAddr) -> bool {
    if let IpAddr::V6(ipv6) = ip {
        let octets = ipv6.octets();
        // ::ffff:0:0/96 — IPv4-mapped IPv6 (conservatively blocked in full).
        if octets[..10] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            && octets[10] == 0xff
            && octets[11] == 0xff
        {
            return true;
        }
    }

    if ip.is_loopback() || ip.is_unspecified() {
        return true;
    }

    BLOCKED_NETWORKS.iter().any(|net| net.contains(&ip))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_unspecified_v4_blocked() {
        // The defect: 0.0.0.0 previously slipped through.
        assert!(is_ip_blocked(IpAddr::V4(Ipv4Addr::UNSPECIFIED)));
        assert!(is_ip_blocked(IpAddr::V4("0.255.0.1".parse().unwrap())));
    }

    #[test]
    fn test_unspecified_v6_blocked() {
        assert!(is_ip_blocked(IpAddr::V6("::".parse().unwrap())));
    }

    #[test]
    fn test_nat64_prefix_blocked() {
        assert!(is_ip_blocked(IpAddr::V6(
            "64:ff9b::192.0.2.1".parse().unwrap()
        )));
        assert!(is_ip_blocked(IpAddr::V6("64:ff9b::1".parse().unwrap())));
        // Just outside the prefix.
        assert!(!is_ip_blocked(IpAddr::V6(
            "64:ff9c::192.0.2.1".parse().unwrap()
        )));
    }

    #[test]
    fn test_public_ips_still_allowed() {
        assert!(!is_ip_blocked(IpAddr::V4("8.8.8.8".parse().unwrap())));
        assert!(!is_ip_blocked(IpAddr::V4("1.1.1.1".parse().unwrap())));
        assert!(!is_ip_blocked(IpAddr::V6(
            "2001:4860:4860::8888".parse().unwrap()
        )));
    }
}
