// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use url::Url;

/// Validate URL to prevent SSRF (Server-Side Request Forgery) attacks
///
/// This function checks if the URL is safe to access by:
/// - Ensuring the URL uses HTTP or HTTPS protocol
/// - Blocking access to internal/private IP addresses
/// - Blocking access to localhost
/// - Blocking access to link-local addresses
/// - Protecting against DNS rebinding attacks
pub fn validate_remote_url(url: &str) -> Result<(), ConfigError> {
    let parsed_url =
        Url::parse(url).map_err(|e| ConfigError::RemoteError(format!("Invalid URL: {}", e)))?;

    // Ensure only HTTP or HTTPS protocols are allowed
    match parsed_url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(ConfigError::RemoteError(format!(
                "Only HTTP and HTTPS protocols are allowed, got: {}",
                scheme
            )))
        }
    }

    // Check if the host is an IP address or hostname
    if let Some(host) = parsed_url.host_str() {
        // Remove IPv6 brackets if present
        let host_clean = host.trim_start_matches('[').trim_end_matches(']');

        // Check for localhost variants
        if is_localhost(host_clean) {
            return Err(ConfigError::RemoteError(
                "Access to localhost is not allowed".to_string(),
            ));
        }

        // Check if it's a hostname (not an IP address)
        if is_hostname(host_clean) {
            // Perform DNS resolution to detect DNS rebinding attacks
            check_dns_rebinding(host_clean)?;
        } else {
            // It's an IP address, check if it's private
            if is_private_ip(host_clean) {
                return Err(ConfigError::RemoteError(
                    "Access to private IP addresses is not allowed".to_string(),
                ));
            }

            // Check for link-local addresses
            if is_link_local(host_clean) {
                return Err(ConfigError::RemoteError(
                    "Access to link-local addresses is not allowed".to_string(),
                ));
            }
        }
    }

    Ok(())
}

/// Check if the host is a hostname (not an IP address)
fn is_hostname(host: &str) -> bool {
    host.parse::<std::net::Ipv4Addr>().is_err() && host.parse::<std::net::Ipv6Addr>().is_err()
}

/// Check for DNS rebinding attacks by resolving the hostname
/// and verifying all resolved IP addresses are not private
fn check_dns_rebinding(host: &str) -> Result<(), ConfigError> {
    use std::net::ToSocketAddrs;

    // Attempt to resolve the hostname
    match (host, 0).to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                let ip = addr.ip();
                let ip_str = ip.to_string();

                // Check if any resolved IP is private
                if is_private_ip(&ip_str) {
                    return Err(ConfigError::RemoteError(format!(
                        "DNS rebinding detected: {} resolves to private IP {}",
                        host, ip
                    )));
                }

                // Check if any resolved IP is link-local
                if is_link_local(&ip_str) {
                    return Err(ConfigError::RemoteError(format!(
                        "DNS rebinding detected: {} resolves to link-local IP {}",
                        host, ip
                    )));
                }

                // Check if any resolved IP is localhost
                if is_localhost(&ip_str) {
                    return Err(ConfigError::RemoteError(format!(
                        "DNS rebinding detected: {} resolves to localhost {}",
                        host, ip
                    )));
                }
            }
        }
        Err(e) => {
            // If DNS resolution fails, it might be a valid hostname that's not currently resolvable
            // We'll allow this but log a warning
            tracing::warn!("Failed to resolve hostname {}: {}", host, e);
        }
    }

    Ok(())
}

/// Check if the host is localhost
fn is_localhost(host: &str) -> bool {
    let host_lower = host.to_lowercase();
    matches!(
        host_lower.as_str(),
        "localhost" | "127.0.0.1" | "::1" | "0.0.0.0" | "[::]"
    )
}

/// Check if the host is a private IP address
fn is_private_ip(host: &str) -> bool {
    // Try to parse as IPv4
    if let Ok(addr) = host.parse::<std::net::Ipv4Addr>() {
        return is_private_ipv4(&addr);
    }

    // Try to parse as IPv6
    if let Ok(addr) = host.parse::<std::net::Ipv6Addr>() {
        return is_private_ipv6(&addr);
    }

    false
}

/// Check if IPv4 address is private
fn is_private_ipv4(addr: &std::net::Ipv4Addr) -> bool {
    let octets = addr.octets();

    // 10.0.0.0/8
    if octets[0] == 10 {
        return true;
    }

    // 172.16.0.0/12
    if octets[0] == 172 && octets[1] >= 16 && octets[1] <= 31 {
        return true;
    }

    // 192.168.0.0/16
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }

    // 169.254.0.0/16 (link-local)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }

    // 100.64.0.0/10 (carrier-grade NAT)
    if octets[0] == 100 && (octets[1] & 0b1100_0000) == 0b0100_0000 {
        return true;
    }

    // 192.0.0.0/24 (IETF Protocol Assignments)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return true;
    }

    // 192.0.2.0/24 (TEST-NET-1, documentation)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
        return true;
    }

    // 198.51.100.0/24 (TEST-NET-2, documentation)
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return true;
    }

    // 203.0.113.0/24 (TEST-NET-3, documentation)
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return true;
    }

    // 240.0.0.0/4 (reserved)
    if octets[0] & 0b1111_0000 == 0b1111_0000 {
        return true;
    }

    false
}

/// Check if IPv6 address is private
fn is_private_ipv6(addr: &std::net::Ipv6Addr) -> bool {
    let segments = addr.segments();

    // fc00::/7 (Unique Local Addresses)
    if segments[0] & 0xfe00 == 0xfc00 {
        return true;
    }

    // fe80::/10 (Link-local)
    if segments[0] & 0xffc0 == 0xfe80 {
        return true;
    }

    // ::/128 (unspecified)
    if addr.is_unspecified() {
        return true;
    }

    // ff00::/8 (multicast)
    if segments[0] & 0xff00 == 0xff00 {
        return true;
    }

    false
}

/// Check if the host is a link-local address
fn is_link_local(host: &str) -> bool {
    // Try to parse as IPv4
    if let Ok(addr) = host.parse::<std::net::Ipv4Addr>() {
        return is_link_local_ipv4(&addr);
    }

    // Try to parse as IPv6
    if let Ok(addr) = host.parse::<std::net::Ipv6Addr>() {
        return is_link_local_ipv6(&addr);
    }

    false
}

/// Check if IPv4 address is link-local
fn is_link_local_ipv4(addr: &std::net::Ipv4Addr) -> bool {
    // 169.254.0.0/16 (link-local)
    let octets = addr.octets();
    octets[0] == 169 && octets[1] == 254
}

/// Check if IPv6 address is link-local
fn is_link_local_ipv6(addr: &std::net::Ipv6Addr) -> bool {
    // fe80::/10 (link-local)
    let segments = addr.segments();
    segments[0] & 0xffc0 == 0xfe80
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_public_url() {
        assert!(validate_remote_url("https://example.com").is_ok());
        assert!(validate_remote_url("http://api.example.com:8080").is_ok());
    }

    #[test]
    fn test_block_localhost() {
        assert!(validate_remote_url("http://localhost").is_err());
        assert!(validate_remote_url("http://127.0.0.1").is_err());
        assert!(validate_remote_url("http://127.0.0.1:8080").is_err());
        assert!(validate_remote_url("http://[::1]").is_err());
    }

    #[test]
    fn test_block_private_ipv4() {
        assert!(validate_remote_url("http://10.0.0.1").is_err());
        assert!(validate_remote_url("http://10.255.255.255").is_err());
        assert!(validate_remote_url("http://172.16.0.1").is_err());
        assert!(validate_remote_url("http://172.31.255.255").is_err());
        assert!(validate_remote_url("http://192.168.1.1").is_err());
        assert!(validate_remote_url("http://192.168.255.255").is_err());
    }

    #[test]
    fn test_block_private_ipv6() {
        assert!(validate_remote_url("http://[fc00::1]").is_err());
        assert!(validate_remote_url("http://[fd00::1]").is_err());
        assert!(validate_remote_url("http://[fe80::1]").is_err());
    }

    #[test]
    fn test_block_link_local() {
        assert!(validate_remote_url("http://169.254.1.1").is_err());
        assert!(validate_remote_url("http://[fe80::1]").is_err());
    }

    #[test]
    fn test_block_invalid_protocol() {
        assert!(validate_remote_url("ftp://example.com").is_err());
        assert!(validate_remote_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn test_block_unspecified() {
        assert!(validate_remote_url("http://0.0.0.0").is_err());
        assert!(validate_remote_url("http://[::]").is_err());
    }
}
