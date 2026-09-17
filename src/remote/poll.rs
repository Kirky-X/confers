// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Polled source abstraction for remote configuration.
//!
//! This module provides the `PolledSource` trait for sources that require
//! periodic polling (HTTP endpoints, databases, etc.) and implements
//! `HttpPolledSource` for HTTP-based configuration sources with ETag
//! and Last-Modified support.
//!
//! # SSRF Protection
//!
//! This module implements comprehensive Server-Side Request Forgery protection:
//! - Blocked IP ranges: private networks, loopback, link-local, documentation ranges
//! - DNS resolution validation: resolved IPs are checked against blocked ranges
//! - DNS rebinding protection: domain names are resolved and validated on the
//!   async poll path (blocking DNS is offloaded via `spawn_blocking`)
//! - Pinned DNS resolution: the HTTP client installs a custom resolver that
//!   re-checks every address reqwest connects to against the same blacklist,
//!   so the validation lookup and the connection cannot observe different
//!   DNS answers (the rebinding TOCTOU window is closed)
//! - Manual redirect following: every redirect hop is re-validated against the
//!   same SSRF rules before the next request is issued
//! - Configurable whitelist: specific domains can be allowed via builder
//! - All blocked attempts return errors with full context
//! - IPv6 support: handles IPv6 addresses and IPv4-mapped IPv6 addresses

use crate::error::{ConfigError, ConfigResult};
use crate::loader::{Format, detect_format_from_content, parse_content};
use crate::remote::circuit_breaker::CircuitBreaker;
use crate::types::{AnnotatedValue, SourceId};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use reqwest::Client;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::sync::LazyLock;

use std::time::Duration;
use tokio::sync::RwLock;

/// Default poll interval when not specified (60 seconds).
pub const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Maximum number of HTTP redirects followed per poll.
///
/// Mirrors reqwest's default limit (10). The HTTP client does NOT follow
/// redirects automatically (`redirect::Policy::none`); each hop is resolved
/// and SSRF-validated manually before the next request is issued, so a
/// redirect cannot bypass the checks applied to the original URL.
const MAX_REDIRECTS: u32 = 10;

static BLOCKED_NETWORKS: LazyLock<Vec<ipnet::IpNet>> = LazyLock::new(|| {
    vec![
        "127.0.0.0/8".parse().unwrap(),
        "10.0.0.0/8".parse().unwrap(),
        "172.16.0.0/12".parse().unwrap(),
        "192.168.0.0/16".parse().unwrap(),
        "169.254.0.0/16".parse().unwrap(),
        "100.64.0.0/10".parse().unwrap(),
        "192.0.2.0/24".parse().unwrap(),
        "198.51.100.0/24".parse().unwrap(),
        "203.0.113.0/24".parse().unwrap(),
        "192.0.0.0/24".parse().unwrap(),
        "fc00::/7".parse().unwrap(),
        "fe80::/10".parse().unwrap(),
    ]
});

/// Check if an IP address is in a blocked range.
pub fn is_ip_blocked(ip: IpAddr) -> bool {
    if let IpAddr::V6(ipv6) = ip {
        let octets = ipv6.octets();
        if octets[..10] == [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
            && octets[10] == 0xff
            && octets[11] == 0xff
        {
            return true;
        }
    }

    if ip.is_loopback() {
        return true;
    }

    BLOCKED_NETWORKS.iter().any(|net| net.contains(&ip))
}

/// Resolve a hostname on tokio's blocking pool, optionally validating every
/// resolved IP against the SSRF blacklist.
///
/// This is the single resolution primitive shared by the poll-path pre-check
/// ([`resolve_host_with_validation`]) and the pinned reqwest DNS resolver
/// (`ValidatingResolver`): the addresses reqwest connects to come from the
/// very same resolution that passed the blacklist checks, so a DNS rebinding
/// attacker cannot show a benign IP to the validation and a private IP to
/// the connection.
///
/// With `enforce_ssrf` set to `false` (direct construction against loopback
/// mock servers only) the resolution is returned unchecked.
///
/// Blocking DNS must never run on the async runtime's worker threads, so the
/// `getaddrinfo` call is offloaded to tokio's blocking pool.
async fn resolve_addrs(
    host: String,
    port: u16,
    enforce_ssrf: bool,
) -> ConfigResult<Vec<SocketAddr>> {
    let addr_string = if port == 0 {
        format!("{}:80", host)
    } else {
        format!("{}:{}", host, port)
    };

    let resolved = tokio::task::spawn_blocking(move || {
        addr_string
            .to_socket_addrs()
            .map(|addrs| addrs.collect::<Vec<SocketAddr>>())
    })
    .await
    .map_err(|_| ConfigError::InvalidValue {
        key: "url".to_string(),
        expected_type: "resolvable hostname".to_string(),
        message: format!("DNS resolution task failed for hostname: {}", host),
    })?
    .map_err(|_| ConfigError::InvalidValue {
        key: "url".to_string(),
        expected_type: "resolvable hostname".to_string(),
        message: format!("Cannot resolve hostname: {}", host),
    })?;

    let addrs: Vec<SocketAddr> = resolved;

    if addrs.is_empty() {
        return Err(ConfigError::InvalidValue {
            key: "url".to_string(),
            expected_type: "resolvable hostname".to_string(),
            message: format!("No addresses resolved for hostname: {}", host),
        });
    }

    if enforce_ssrf {
        for addr in &addrs {
            if is_ip_blocked(addr.ip()) {
                // SSRF attempt detected - return error without logging
                return Err(ConfigError::InvalidValue {
                    key: "url".to_string(),
                    expected_type: "public IP".to_string(),
                    message:
                        "SSRF attempt detected: resolved IP address is in a blocked private range"
                            .to_string(),
                });
            }
        }
    }

    Ok(addrs)
}

/// Resolve a hostname and check all resolved IPs against blocked ranges.
///
/// This provides DNS rebinding protection by validating that ALL resolved IPs
/// are safe to connect to. If any IP is blocked, the connection is rejected.
///
/// # Arguments
///
/// * `host` - The hostname to resolve
/// * `port` - The port to use for resolution
///
/// # Returns
///
/// Returns `Ok(Vec<IpAddr>)` with all resolved IPs if all are safe,
/// or an error if any IP is blocked.
async fn resolve_host_with_validation(host: &str, port: u16) -> ConfigResult<Vec<IpAddr>> {
    Ok(resolve_addrs(host.to_string(), port, true)
        .await?
        .into_iter()
        .map(|addr| addr.ip())
        .collect())
}

/// Custom reqwest DNS resolver that pins SSRF-validated addresses.
///
/// reqwest performs its own DNS lookup when establishing a connection, which
/// is independent of the pre-connect validation in `do_poll`. Without
/// pinning, an attacker controlling the DNS answers for a domain could return
/// a benign IP to the validation lookup and a private IP to reqwest's lookup
/// — the classic DNS-rebinding TOCTOU window. Installing this resolver on
/// the client (see [`HttpPolledSourceBuilder::build`]) closes that window:
/// the addresses reqwest connects to are produced by the same resolution
/// that was checked against the SSRF blacklist, so the two can no longer
/// diverge.
///
/// The blacklist check runs unconditionally for every resolved address: a
/// domain whitelist only skips the domain-level DNS pre-check (see
/// [`is_domain_whitelisted`]), it never exempts private IPs.
struct ValidatingResolver {
    /// Whether resolved addresses must pass the SSRF blacklist checks.
    ///
    /// `false` only on the direct-construction path used by loopback
    /// integration tests; addresses are then resolved and returned unchecked.
    enforce_ssrf: bool,
}

impl Resolve for ValidatingResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        let enforce_ssrf = self.enforce_ssrf;
        Box::pin(async move {
            // The port is irrelevant for name resolution here; hyper's
            // connector overrides it with the port from the request URL.
            let addrs = resolve_addrs(host, 0, enforce_ssrf).await?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// Check whether a domain matches the SSRF whitelist.
///
/// Supports exact entries, subdomain entries (`example.com` matches
/// `api.example.com`) and explicit wildcards (`*.example.com` matches
/// `sub.example.com` but not `example.com`).
fn is_domain_whitelisted(domain: &str, allowed_domains: &[String]) -> bool {
    allowed_domains.iter().any(|allowed| {
        // Exact match
        if allowed == domain {
            return true;
        }
        // Wildcard match: *.example.com matches sub.example.com
        if let Some(suffix) = allowed.strip_prefix("*.") {
            // Domain must be a proper subdomain (e.g. sub.example.com, not example.com)
            return domain.ends_with(&format!(".{suffix}"));
        }
        // Subdomain match for non-wildcard entries (e.g. "example.com" matches "sub.example.com")
        domain.ends_with(&format!(".{allowed}"))
    })
}

/// Apply all DNS-free SSRF checks to an already-parsed URL.
///
/// 1. Only HTTPS URLs are allowed
/// 2. The URL must have a host
/// 3. Direct IP hosts are validated against blocked ranges
///
/// Domain hosts only need DNS-based validation (see `validate_url_full`);
/// the whitelist only decides whether that DNS check is performed, so it is
/// not consulted here.
fn validate_url_parts(parsed: &url::Url) -> ConfigResult<()> {
    // Only allow HTTPS by default for security
    if parsed.scheme() != "https" {
        // Non-HTTPS URL rejected - return error without logging
        return Err(ConfigError::InvalidValue {
            key: "url".to_string(),
            expected_type: "https URL".to_string(),
            message: "Only HTTPS URLs are allowed for remote configuration".to_string(),
        });
    }

    match parsed.host() {
        Some(url::Host::Ipv4(ip)) => {
            if is_ip_blocked(IpAddr::V4(ip)) {
                // Private IPv4 rejected - return error without logging
                return Err(ConfigError::InvalidValue {
                    key: "url".to_string(),
                    expected_type: "public IP".to_string(),
                    message: "Connection to private/internal IP addresses is not allowed"
                        .to_string(),
                });
            }
        }
        Some(url::Host::Ipv6(ip)) => {
            if is_ip_blocked(IpAddr::V6(ip)) {
                // Private IPv6 rejected - return error without logging
                return Err(ConfigError::InvalidValue {
                    key: "url".to_string(),
                    expected_type: "public IP".to_string(),
                    message: "Connection to private/internal IP addresses is not allowed"
                        .to_string(),
                });
            }
        }
        Some(url::Host::Domain(_)) => {}
        None => {
            return Err(ConfigError::InvalidValue {
                key: "url".to_string(),
                expected_type: "valid URL with host".to_string(),
                message: "URL must have a host".to_string(),
            });
        }
    }

    Ok(())
}

/// Validate a URL string for security (SSRF protection) — DNS-free checks only.
///
/// Parses the URL and applies the static SSRF checks (HTTPS-only scheme, host
/// presence, blocked-range checks for direct IP hosts). DNS resolution for
/// domain hosts is NOT performed here; it happens asynchronously on the poll
/// path via `validate_url_full`.
///
/// Returns the parsed URL on success.
fn validate_url(url: &str) -> ConfigResult<url::Url> {
    let parsed = url::Url::parse(url).map_err(|_| ConfigError::InvalidValue {
        key: "url".to_string(),
        expected_type: "valid URL".to_string(),
        message: "Invalid URL format".to_string(),
    })?;

    validate_url_parts(&parsed)?;

    Ok(parsed)
}

/// Full SSRF validation of a parsed URL on the async poll path.
///
/// Applies the DNS-free static checks plus DNS resolution with blocked-IP
/// validation for non-whitelisted domain hosts (DNS rebinding protection).
/// Whitelisted domains skip DNS validation entirely — the whitelist only
/// bypasses this domain-level pre-check; the client's pinned resolver
/// (`ValidatingResolver`) still validates every address at connection
/// time, so a whitelisted domain that resolves to a private IP is rejected
/// when connecting.
async fn validate_url_full(parsed: &url::Url, allowed_domains: &[String]) -> ConfigResult<()> {
    validate_url_parts(parsed)?;

    if let Some(url::Host::Domain(domain)) = parsed.host() {
        let domain = domain.to_string();
        if !is_domain_whitelisted(&domain, allowed_domains) {
            let port = parsed.port().unwrap_or(443);
            resolve_host_with_validation(&domain, port).await?;
        }
    }

    Ok(())
}

/// Resolve and statically validate one redirect hop.
///
/// Pure helper (no DNS, no I/O) so the per-hop redirect rules are unit-testable
/// in isolation. `location` is resolved against `current` per RFC 3986
/// (relative references such as `/path` or `../other` are allowed), then the
/// same DNS-free SSRF checks as for the original URL are applied: HTTPS-only
/// scheme, host presence and blocked-IP ranges. DNS-based validation for
/// domain hops is performed separately on the async path (see
/// `validate_url_full`).
fn validate_redirect_hop(current: &url::Url, location: &str) -> ConfigResult<url::Url> {
    let next = current
        .join(location)
        .map_err(|_| ConfigError::InvalidValue {
            key: "url".to_string(),
            expected_type: "valid redirect Location URL".to_string(),
            message: format!("Invalid redirect Location: {location}"),
        })?;

    validate_url_parts(&next)?;

    Ok(next)
}

/// Trait for polled configuration sources.
///
/// Sources implementing this trait will be polled at regular intervals
/// to fetch the latest configuration values.
#[async_trait]
pub trait PolledSource: Send + Sync {
    /// Poll the source for the latest configuration.
    async fn poll(&self) -> ConfigResult<AnnotatedValue>;

    /// Get the poll interval for this source.
    fn poll_interval(&self) -> Option<Duration>;

    /// Get the source identifier.
    fn source_id(&self) -> SourceId;
}

/// HTTP-polled configuration source.
///
/// Fetches configuration from an HTTP endpoint with support for:
/// - ETag-based conditional requests (If-None-Match)
/// - Last-Modified-based conditional requests (If-Modified-Since)
/// - Configurable poll intervals
/// - Automatic format detection
/// - SSRF protection with configurable domain whitelist
///
/// # SSRF Protection
///
/// By default, connections to the following are blocked:
/// - Private IP ranges (10.x.x.x, 172.16-31.x.x, 192.168.x.x)
/// - Loopback addresses (127.x.x.x, ::1)
/// - Link-local addresses (169.254.x.x, fe80::/10)
/// - Documentation IP ranges (192.0.2.x/24, etc.)
/// - IPv4-mapped IPv6 addresses (::ffff:x.x.x.x)
///
/// Static checks (scheme, direct-IP hosts) run at build time; DNS names are
/// resolved and validated on every poll request, and redirects are followed
/// manually with the same per-hop validation, so a redirect cannot bypass
/// the SSRF rules. The HTTP client additionally pins DNS resolution
/// (`ValidatingResolver`): addresses reqwest connects to come from the
/// same resolution that was checked against the blacklist, so a rebinding
/// attacker cannot show a different IP to the connection than to the
/// validation.
///
/// # Examples
///
/// ```
/// use confers::remote::HttpPolledSourceBuilder;
/// use std::time::Duration;
///
/// let source = HttpPolledSourceBuilder::new()
///     .url("https://config.example.com/app.json")
///     .interval(Duration::from_secs(30))
///     .allowed_domain("config.example.com")
///     .allowed_domain("cdn.example.com")
///     .build()
///     .unwrap();
/// ```
///
/// # Lock Contention Optimization
///
/// This implementation uses atomics for ETag/Modified tracking to minimize
/// lock contention in high-concurrency scenarios. Only the cached value
/// is protected by a RwLock, which is held for the minimal time necessary.
#[derive(Debug)]
pub struct HttpPolledSource {
    url: Arc<str>,
    interval: Duration,
    client: Client,
    format: Option<Format>,
    /// Domains whitelisted for SSRF checks (exact, subdomain or wildcard).
    allowed_domains: Arc<[String]>,
    /// Whether per-request SSRF validation is enforced on the poll path.
    ///
    /// `true` for sources built via [`HttpPolledSourceBuilder`]. Direct
    /// construction (tests against local mock servers) may set it to `false`
    /// because loopback HTTP endpoints are rejected by the SSRF rules.
    enforce_ssrf: bool,
    cached: RwLock<Option<AnnotatedValue>>,
    last_etag: ArcSwap<Option<String>>,
    last_modified: ArcSwap<Option<String>>,
    source_id: SourceId,
    circuit_breaker: std::sync::Mutex<CircuitBreaker>,
}

/// Builder for `HttpPolledSource`.
pub struct HttpPolledSourceBuilder {
    url: Option<String>,
    interval: Option<Duration>,
    format: Option<Format>,
    timeout: Option<Duration>,
    allowed_domains: Vec<String>,
    cb_threshold: Option<u32>,
    cb_base_delay: Option<Duration>,
    cb_max_delay: Option<Duration>,
}

impl HttpPolledSourceBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            url: None,
            interval: None,
            format: None,
            timeout: None,
            allowed_domains: Vec::new(),
            cb_threshold: None,
            cb_base_delay: None,
            cb_max_delay: None,
        }
    }

    /// Set the URL of the remote configuration endpoint.
    ///
    /// The URL must use HTTPS. Direct-IP hosts are checked against blocked
    /// ranges at build time; domain hosts are resolved and validated on the
    /// async poll path (blocking DNS must not run inside `build()`).
    ///
    /// # SSRF Protection
    ///
    /// URLs pointing to private IPs, localhost, or documentation ranges
    /// will be rejected at build time.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the poll interval.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Set the configuration format (auto-detected if not specified).
    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add a domain to the allowed whitelist.
    ///
    /// Whitelisted domains skip the domain-level DNS pre-check on the poll
    /// path (see `validate_url_full`). The IP blacklist itself is NOT
    /// relaxed: the client's pinned DNS resolver checks every resolved
    /// address unconditionally, so a whitelisted domain that resolves to a
    /// private/loopback IP is still rejected at connection time.
    ///
    /// Supports:
    /// - Exact match: `internal.example.com`
    /// - Subdomain wildcard: `example.com` matches `api.example.com`
    /// - Explicit wildcard: `*.example.com` (via prefix check)
    ///
    /// # Security Note
    ///
    /// Use whitelisting sparingly. The whitelist only suppresses the
    /// domain-level pre-check; it never allows connections to blocked IP
    /// ranges.
    pub fn allowed_domain(mut self, domain: impl Into<String>) -> Self {
        self.allowed_domains.push(domain.into());
        self
    }

    /// Add multiple domains to the allowed whitelist.
    pub fn allowed_domains(mut self, domains: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for domain in domains {
            self.allowed_domains.push(domain.into());
        }
        self
    }

    /// Set the circuit breaker failure threshold.
    ///
    /// After this many consecutive failures, the circuit opens and polls
    /// are skipped until the backoff timeout elapses. Default: 5.
    pub fn circuit_breaker_threshold(mut self, failures: u32) -> Self {
        self.cb_threshold = Some(failures);
        self
    }

    /// Set the circuit breaker base delay for exponential backoff.
    /// Default: 1 second.
    pub fn circuit_breaker_base_delay(mut self, delay: Duration) -> Self {
        self.cb_base_delay = Some(delay);
        self
    }

    /// Set the circuit breaker maximum backoff delay.
    /// Default: 60 seconds.
    pub fn circuit_breaker_max_delay(mut self, delay: Duration) -> Self {
        self.cb_max_delay = Some(delay);
        self
    }

    /// Build the `HttpPolledSource`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL is missing
    /// - URL uses non-HTTPS scheme
    /// - URL host is a blocked private IP
    ///
    /// Domain hosts are resolved on the async poll path (see
    /// `validate_url_full`); `build()` deliberately performs no blocking
    /// DNS resolution.
    pub fn build(self) -> ConfigResult<HttpPolledSource> {
        let url = self.url.ok_or_else(|| ConfigError::InvalidValue {
            key: "url".to_string(),
            expected_type: "string".to_string(),
            message: "URL is required".to_string(),
        })?;

        // Validate URL for security (DNS-free SSRF checks; DNS resolution
        // with blocked-IP validation runs asynchronously on every poll, which
        // also keeps rebinding protection fresh).
        validate_url(&url)?;

        let url_arc: Arc<str> = url.clone().into();
        let source_id = SourceId::new(format!("http:{}", url_arc));

        // Build HTTP client with TLS enabled by default. Automatic redirect
        // following is disabled: redirects are followed manually in `do_poll`
        // so every hop passes the same SSRF validation as the original URL.
        // The pinned DNS resolver makes reqwest connect to addresses from the
        // same validated resolution as the poll-path pre-check, eliminating
        // the DNS-rebinding TOCTOU window between validation and connection.
        let mut client_builder = Client::builder()
            .use_rustls_tls()
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(Arc::new(ValidatingResolver { enforce_ssrf: true }));

        if let Some(timeout) = self.timeout {
            client_builder = client_builder.timeout(timeout);
        }

        let client = client_builder
            .build()
            .map_err(|_e| ConfigError::RemoteUnavailable {
                error_type: "ClientBuild".to_string(),
                retryable: false,
            })?;

        // Build circuit breaker with configured or default values
        let mut cb = CircuitBreaker::new();
        if let Some(threshold) = self.cb_threshold {
            cb = cb.with_threshold(threshold);
        }
        if let Some(base_delay) = self.cb_base_delay {
            cb = cb.with_base_delay(base_delay);
        }
        if let Some(max_delay) = self.cb_max_delay {
            cb = cb.with_max_delay(max_delay);
        }

        Ok(HttpPolledSource {
            url: url_arc,
            interval: self.interval.unwrap_or(DEFAULT_POLL_INTERVAL),
            client,
            format: self.format,
            allowed_domains: self.allowed_domains.into(),
            enforce_ssrf: true,
            cached: RwLock::new(None),
            last_etag: ArcSwap::new(Arc::new(None)),
            last_modified: ArcSwap::new(Arc::new(None)),
            source_id,
            circuit_breaker: std::sync::Mutex::new(cb),
        })
    }
}

impl Default for HttpPolledSourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolledSource for HttpPolledSource {
    /// Poll the HTTP endpoint for configuration.
    ///
    /// Uses ETag and Last-Modified headers for conditional requests.
    /// Returns cached value on 304 Not Modified responses.
    ///
    /// The integrated circuit breaker skips polls when too many
    /// consecutive failures have occurred, returning an error immediately
    /// without making an HTTP request.
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        super::record_fetch_metrics(&self.source_id.clone(), self.poll_with_circuit_breaker()).await
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(self.interval)
    }

    fn source_id(&self) -> SourceId {
        self.source_id.clone()
    }
}

impl HttpPolledSource {
    /// Poll with circuit-breaker accounting (the original `poll` body).
    async fn poll_with_circuit_breaker(&self) -> ConfigResult<AnnotatedValue> {
        // Check circuit breaker before making any HTTP request
        {
            let mut cb = self.circuit_breaker.lock().unwrap();
            if !cb.can_execute() {
                return Err(ConfigError::RemoteUnavailable {
                    error_type: "CircuitBreakerOpen".to_string(),
                    retryable: false,
                });
            }
        }

        let result = self.do_poll().await;

        // Record outcome in circuit breaker
        {
            let mut cb = self.circuit_breaker.lock().unwrap();
            match &result {
                Ok(_) => cb.record_success(),
                Err(_) => cb.record_failure(),
            }
        }

        result
    }

    /// Internal poll implementation (without circuit breaker logic).
    ///
    /// Redirects are followed manually (up to [`MAX_REDIRECTS`] hops): the
    /// HTTP client is built with `redirect::Policy::none`, and every hop —
    /// including the original URL — is SSRF-validated before its request is
    /// issued, so a redirect cannot bypass the checks applied at build time.
    async fn do_poll(&self) -> ConfigResult<AnnotatedValue> {
        let mut current_url: url::Url =
            self.url.parse().map_err(|_| ConfigError::InvalidValue {
                key: "url".to_string(),
                expected_type: "valid URL".to_string(),
                message: "Invalid URL format".to_string(),
            })?;

        let mut redirects_followed: u32 = 0;

        loop {
            if self.enforce_ssrf {
                // Full per-request validation: static checks plus DNS
                // resolution with blocked-IP validation for domain hosts.
                validate_url_full(&current_url, &self.allowed_domains).await?;
            }

            let mut request = self.client.get(current_url.clone());

            if let Some(etag) = self.last_etag.load().as_ref() {
                request = request.header("If-None-Match", etag.as_str());
            }

            if let Some(modified) = self.last_modified.load().as_ref() {
                request = request.header("If-Modified-Since", modified.as_str());
            }

            let response = request
                .send()
                .await
                .map_err(|e| ConfigError::RemoteUnavailable {
                    error_type: std::any::type_name::<reqwest::Error>().to_string(),
                    retryable: is_retryable_error(&e),
                })?;

            let status = response.status();

            if status == reqwest::StatusCode::NOT_MODIFIED {
                if let Some(cached) = self.cached.read().await.as_ref() {
                    return Ok(cached.clone());
                }
                return Err(ConfigError::RemoteUnavailable {
                    error_type: "NoCachedValue".to_string(),
                    retryable: false,
                });
            }

            if status.is_redirection() {
                let location = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .ok_or_else(|| ConfigError::RemoteUnavailable {
                        error_type: "MissingRedirectLocation".to_string(),
                        retryable: false,
                    })?
                    .to_str()
                    .map_err(|_| ConfigError::RemoteUnavailable {
                        error_type: "InvalidRedirectLocation".to_string(),
                        retryable: false,
                    })?
                    .to_owned();

                if redirects_followed >= MAX_REDIRECTS {
                    return Err(ConfigError::RemoteUnavailable {
                        error_type: "TooManyRedirects".to_string(),
                        retryable: false,
                    });
                }
                redirects_followed += 1;

                // Resolve the next hop (relative Locations are resolved
                // against the current URL) and validate it against the SSRF
                // rules BEFORE issuing the next request. The full DNS-based
                // validation then runs at the top of the loop, so a hop to a
                // domain that resolves to a blocked IP is rejected too.
                current_url = if self.enforce_ssrf {
                    validate_redirect_hop(&current_url, &location)?
                } else {
                    current_url
                        .join(&location)
                        .map_err(|_| ConfigError::RemoteUnavailable {
                            error_type: "InvalidRedirectLocation".to_string(),
                            retryable: false,
                        })?
                };
                continue;
            }

            if !status.is_success() {
                return Err(ConfigError::RemoteUnavailable {
                    error_type: format!("HTTP_{}", status.as_u16()),
                    retryable: status.is_server_error() || status.as_u16() == 429,
                });
            }

            if let Some(etag) = response.headers().get("etag")
                && let Ok(etag_str) = etag.to_str()
            {
                self.last_etag.store(Arc::new(Some(etag_str.to_string())));
            }

            if let Some(modified) = response.headers().get("last-modified")
                && let Ok(modified_str) = modified.to_str()
            {
                self.last_modified
                    .store(Arc::new(Some(modified_str.to_string())));
            }

            let body = response
                .text()
                .await
                .map_err(|e| ConfigError::RemoteUnavailable {
                    error_type: std::any::type_name::<reqwest::Error>().to_string(),
                    retryable: is_retryable_error(&e),
                })?;

            let format = self
                .format
                .unwrap_or_else(|| detect_format_from_content(&body).unwrap_or(Format::Json));

            let source = self.source_id.clone();
            let value = parse_remote_content(&body, format, source)?;

            *self.cached.write().await = Some(value.clone());

            return Ok(value);
        }
    }
}

/// Parse content from a remote source using the unified parser.
fn parse_remote_content(
    content: &str,
    format: Format,
    source: SourceId,
) -> ConfigResult<AnnotatedValue> {
    // Use the unified parse_content from loader.rs, which handles all formats
    // consistently. The Format enum already exists in loader.rs.
    parse_content(content, format, source, None)
}

/// Check if a reqwest error is likely retryable.
fn is_retryable_error(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // SSRF Protection Tests (9.1.7)
    // =============================================================================

    #[test]
    fn test_is_ip_blocked_loopback_v4() {
        // 127.0.0.0/8 - all loopback
        assert!(is_ip_blocked(IpAddr::V4("127.0.0.1".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V4(
            "127.255.255.255".parse().unwrap()
        )));
        assert!(is_ip_blocked(IpAddr::V4("127.0.0.0".parse().unwrap())));
    }

    #[test]
    fn test_is_ip_blocked_private_v4() {
        // 10.0.0.0/8
        assert!(is_ip_blocked(IpAddr::V4("10.0.0.1".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V4("10.255.255.255".parse().unwrap())));
        // 172.16.0.0/12
        assert!(is_ip_blocked(IpAddr::V4("172.16.0.0".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V4("172.31.255.255".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V4("172.20.0.1".parse().unwrap())));
        // 172.15.x.x and 172.32.x.x are NOT blocked
        assert!(!is_ip_blocked(IpAddr::V4("172.15.0.1".parse().unwrap())));
        assert!(!is_ip_blocked(IpAddr::V4("172.32.0.1".parse().unwrap())));
        // 192.168.0.0/16
        assert!(is_ip_blocked(IpAddr::V4("192.168.0.1".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V4(
            "192.168.255.255".parse().unwrap()
        )));
    }

    #[test]
    fn test_is_ip_blocked_link_local_v4() {
        // 169.254.0.0/16
        assert!(is_ip_blocked(IpAddr::V4("169.254.0.0".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V4(
            "169.254.255.255".parse().unwrap()
        )));
    }

    #[test]
    fn test_is_ip_blocked_carrier_nat_v4() {
        // 100.64.0.0/10
        assert!(is_ip_blocked(IpAddr::V4("100.64.0.1".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V4(
            "100.127.255.255".parse().unwrap()
        )));
        // 100.0-63 and 100.128+ are NOT blocked
        assert!(!is_ip_blocked(IpAddr::V4("100.0.0.1".parse().unwrap())));
        assert!(!is_ip_blocked(IpAddr::V4("100.128.0.1".parse().unwrap())));
    }

    #[test]
    fn test_is_ip_blocked_documentation_v4() {
        // 192.0.2.0/24 (DOC-1)
        assert!(is_ip_blocked(IpAddr::V4("192.0.2.1".parse().unwrap())));
        // 198.51.100.0/24 (DOC-2)
        assert!(is_ip_blocked(IpAddr::V4("198.51.100.1".parse().unwrap())));
        // 203.0.113.0/24 (DOC-3)
        assert!(is_ip_blocked(IpAddr::V4("203.0.113.1".parse().unwrap())));
        // 192.0.0.0/24 (IETF Protocol)
        assert!(is_ip_blocked(IpAddr::V4("192.0.0.1".parse().unwrap())));
    }

    #[test]
    fn test_is_ip_blocked_public_v4() {
        // Public IPs should not be blocked
        assert!(!is_ip_blocked(IpAddr::V4("8.8.8.8".parse().unwrap())));
        assert!(!is_ip_blocked(IpAddr::V4("1.1.1.1".parse().unwrap())));
        assert!(!is_ip_blocked(IpAddr::V4("93.184.216.34".parse().unwrap()))); // example.com
        assert!(!is_ip_blocked(IpAddr::V4("52.94.236.248".parse().unwrap()))); // AWS
    }

    #[test]
    fn test_is_ip_blocked_loopback_v6() {
        // ::1/128
        assert!(is_ip_blocked(IpAddr::V6("::1".parse().unwrap())));
        // ::0/128 is not blocked
        assert!(!is_ip_blocked(IpAddr::V6("::0".parse().unwrap())));
    }

    #[test]
    fn test_is_ip_blocked_unique_local_v6() {
        // fc00::/7
        assert!(is_ip_blocked(IpAddr::V6("fc00::1".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V6("fd00::1".parse().unwrap())));
        // fdFF::/8 is the random local address range
        assert!(is_ip_blocked(IpAddr::V6("fdff::1".parse().unwrap())));
        // fe00::/7 is NOT unique local (fe00 is)
        assert!(!is_ip_blocked(IpAddr::V6("fe00::1".parse().unwrap())));
    }

    #[test]
    fn test_is_ip_blocked_link_local_v6() {
        // fe80::/10
        assert!(is_ip_blocked(IpAddr::V6("fe80::1".parse().unwrap())));
        assert!(is_ip_blocked(IpAddr::V6(
            "fe80:ffff:ffff:ffff::".parse().unwrap()
        )));
        // fe81:: is also blocked (still in fe80::/10)
        assert!(is_ip_blocked(IpAddr::V6("fe81::1".parse().unwrap())));
        // fe7f:: is NOT blocked (just outside fe80::/10)
        assert!(!is_ip_blocked(IpAddr::V6("fe7f::1".parse().unwrap())));
    }

    #[test]
    fn test_is_ip_blocked_ipv4_mapped_v6() {
        // IPv4-mapped IPv6 addresses (::ffff:0:0/96)
        assert!(is_ip_blocked(IpAddr::V6(
            "::ffff:127.0.0.1".parse().unwrap()
        )));
        assert!(is_ip_blocked(IpAddr::V6("::ffff:0:0".parse().unwrap())));
        // IPv4-mapped public IPs are still blocked
        assert!(is_ip_blocked(IpAddr::V6("::ffff:8.8.8.8".parse().unwrap())));
    }

    #[test]
    fn test_is_ip_blocked_public_v6() {
        // Public IPv6 addresses should not be blocked
        assert!(!is_ip_blocked(IpAddr::V6(
            "2001:4860:4860::8888".parse().unwrap()
        ))); // Google DNS
        assert!(!is_ip_blocked(IpAddr::V6(
            "2606:4700:4700::1111".parse().unwrap()
        ))); // Cloudflare DNS
    }

    // =============================================================================
    // URL Validation Tests (9.1.7)
    // =============================================================================

    #[test]
    fn test_validate_url_rejects_non_https() {
        let result = validate_url("http://example.com/config.json");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::InvalidValue { .. }));
    }

    #[test]
    fn test_validate_url_rejects_private_ipv4() {
        // 127.0.0.1
        let result = validate_url("https://127.0.0.1/config.json");
        assert!(result.is_err());
        // 10.x.x.x
        let result = validate_url("https://10.0.0.1/config.json");
        assert!(result.is_err());
        // 192.168.x.x
        let result = validate_url("https://192.168.1.1/config.json");
        assert!(err_if_blocked(&result));
        assert!(result.is_err());
        // 172.16.x.x
        let result = validate_url("https://172.16.0.1/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_rejects_private_ipv6() {
        // ::1
        let result = validate_url("https://[::1]/config.json");
        assert!(result.is_err());
        // fe80:: (link-local)
        let result = validate_url("https://[fe80::1]/config.json");
        assert!(result.is_err());
        // fc00:: (unique local)
        let result = validate_url("https://[fc00::1]/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_rejects_ipv4_mapped() {
        // ::ffff:127.0.0.1
        let result = validate_url("https://[::ffff:127.0.0.1]/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_rejects_documentation_ips() {
        let result = validate_url("https://192.0.2.1/config.json");
        assert!(result.is_err());
        let result = validate_url("https://198.51.100.1/config.json");
        assert!(result.is_err());
        let result = validate_url("https://203.0.113.1/config.json");
        assert!(result.is_err());
        let result = validate_url("https://192.0.0.1/config.json");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_accepts_public_ips() {
        // Public IP hosts pass the static SSRF checks.
        let result = validate_url("https://8.8.8.8/config.json");
        assert!(result.is_ok(), "public IP host should pass: {result:?}");
    }

    /// Issue #348: `build()` no longer resolves DNS, so `validate_url` is a
    /// pure DNS-free check: an unresolvable domain passes the static checks
    /// and its DNS validation happens asynchronously on the poll path.
    #[test]
    fn test_validate_url_static_accepts_unresolvable_domains() {
        // ".invalid" is guaranteed not to resolve (RFC 2606).
        let result = validate_url("https://nonexistent-host-for-tests.invalid/config.json");
        assert!(
            result.is_ok(),
            "static validation must not require DNS: {result:?}"
        );
    }

    /// Issue #348: full poll-path validation rejects domain hosts that
    /// resolve to blocked IPs (DNS rebinding protection), and whitelisted
    /// domains skip the DNS check entirely.
    #[tokio::test]
    async fn test_validate_url_full_dns_rebinding_protection() {
        // "localhost" resolves (hosts file) to a loopback IP → blocked.
        let parsed = url::Url::parse("https://localhost/config.json").unwrap();
        let result = validate_url_full(&parsed, &[]).await;
        assert!(result.is_err(), "localhost must be rejected: {result:?}");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("SSRF") || err.contains("resolve"),
            "expected SSRF or DNS error, got: {err}"
        );

        // The same domain, whitelisted, skips DNS validation entirely.
        let result = validate_url_full(&parsed, &["localhost".to_string()]).await;
        assert!(result.is_ok(), "whitelisted domain must pass: {result:?}");

        // Direct-IP hosts pass full validation without any DNS round trip.
        let parsed = url::Url::parse("https://8.8.8.8/config.json").unwrap();
        assert!(validate_url_full(&parsed, &[]).await.is_ok());
    }

    /// Issue #348: DNS resolution is offloaded to the blocking pool but the
    /// SSRF semantics are unchanged — resolved private IPs are rejected.
    #[tokio::test]
    async fn test_resolve_host_with_validation_blocks_loopback() {
        let result = resolve_host_with_validation("localhost", 443).await;
        assert!(
            result.is_err(),
            "a hostname resolving to loopback must be rejected: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_resolve_host_with_validation_public() {
        // Test with a well-known public DNS
        // Note: This test requires network access. If it fails, the host doesn't resolve.
        let result = resolve_host_with_validation("example.com", 443).await;
        if let Ok(ips) = result {
            assert!(!ips.is_empty());
            for ip in &ips {
                assert!(
                    !is_ip_blocked(*ip),
                    "example.com resolved to a blocked IP: {}",
                    ip
                );
            }
        }
        // If network is unavailable, test is skipped
    }

    // =============================================================================
    // ValidatingResolver Tests (pinned DNS resolution, rebinding TOCTOU fix)
    // =============================================================================

    /// The pinned resolver rejects domains whose resolution contains a
    /// blocked IP. This is the check reqwest runs at connection time on the
    /// very addresses it will connect to, so the second (unvalidated) DNS
    /// lookup that enabled the rebinding TOCTOU no longer exists.
    #[tokio::test]
    async fn test_validating_resolver_rejects_domain_resolving_to_loopback() {
        let resolver = ValidatingResolver { enforce_ssrf: true };
        let name: Name = "localhost".parse().expect("valid DNS name");
        // Match instead of unwrap_err: the resolved-addr iterator is not Debug.
        let err = match resolver.resolve(name).await {
            Ok(_) => panic!("a domain resolving to loopback must be rejected by the resolver"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("SSRF") || err.contains("resolve"),
            "expected SSRF or DNS error from the resolver, got: {err}"
        );
    }

    /// With enforcement disabled (direct construction against loopback mock
    /// servers only) the resolver resolves without any IP validation.
    #[tokio::test]
    async fn test_validating_resolver_passthrough_when_not_enforced() {
        let resolver = ValidatingResolver {
            enforce_ssrf: false,
        };
        let name: Name = "localhost".parse().expect("valid DNS name");
        let mut addrs = resolver
            .resolve(name)
            .await
            .expect("passthrough resolver must not validate resolved IPs");
        assert!(
            addrs.next().is_some(),
            "passthrough resolution must return addresses"
        );
    }

    /// The pinned resolver accepts a public domain and only returns
    /// validated addresses. Requires network access; skipped when the host
    /// does not resolve (same convention as
    /// `test_resolve_host_with_validation_public`).
    #[tokio::test]
    async fn test_validating_resolver_accepts_public_domain() {
        let resolver = ValidatingResolver { enforce_ssrf: true };
        let name: Name = "example.com".parse().expect("valid DNS name");
        if let Ok(addrs) = resolver.resolve(name).await {
            for addr in addrs {
                assert!(
                    !is_ip_blocked(addr.ip()),
                    "example.com resolved to a blocked IP: {}",
                    addr.ip()
                );
            }
        }
        // If network is unavailable, test is skipped
    }

    /// End-to-end wiring proof: a whitelisted domain skips the domain-level
    /// pre-check in `validate_url_full`, but the builder-built client's
    /// pinned resolver still blocks the poll because `localhost` resolves to
    /// loopback. Before the pinned resolver existed this poll would have
    /// connected straight to the loopback address.
    #[tokio::test]
    async fn test_poll_whitelisted_domain_still_blocked_by_resolver() {
        let source = HttpPolledSourceBuilder::new()
            .url("https://localhost/config.json")
            .allowed_domain("localhost")
            .build()
            .expect("build passes: domain hosts have no static IP checks to fail");

        let result = source.poll().await;
        assert!(
            matches!(result, Err(ConfigError::RemoteUnavailable { .. })),
            "pinned resolver must block the connection to a loopback-resolving domain"
        );
    }

    // =============================================================================
    // Whitelist Matching Tests (pure helper)
    // =============================================================================

    #[test]
    fn test_is_domain_whitelisted_exact_match() {
        assert!(is_domain_whitelisted(
            "internal.example.com",
            &["internal.example.com".to_string()]
        ));
        assert!(!is_domain_whitelisted(
            "other.example.com",
            &["internal.example.com".to_string()]
        ));
    }

    #[test]
    fn test_is_domain_whitelisted_subdomain_match() {
        // "example.com" matches "api.example.com" but not "example.com" lookalikes
        assert!(is_domain_whitelisted(
            "api.example.com",
            &["example.com".to_string()]
        ));
        assert!(!is_domain_whitelisted(
            "notexample.com",
            &["example.com".to_string()]
        ));
    }

    #[test]
    fn test_is_domain_whitelisted_wildcard() {
        let allowed = vec!["*.example.com".to_string()];
        assert!(is_domain_whitelisted("sub.example.com", &allowed));
        // A wildcard entry does NOT match the bare domain.
        assert!(!is_domain_whitelisted("example.com", &allowed));
    }

    // =============================================================================
    // Redirect Hop Validation Tests (Issue #347, pure helper)
    // =============================================================================

    #[test]
    fn test_validate_redirect_hop_resolves_relative_location() {
        let current = url::Url::parse("https://config.example.com/a/b.json").unwrap();
        let next = validate_redirect_hop(&current, "../cfg.json").unwrap();
        assert_eq!(next.as_str(), "https://config.example.com/cfg.json");

        let next = validate_redirect_hop(&current, "/rooted.json").unwrap();
        assert_eq!(next.as_str(), "https://config.example.com/rooted.json");
    }

    #[test]
    fn test_validate_redirect_hop_accepts_absolute_https() {
        let current = url::Url::parse("https://config.example.com/a").unwrap();
        let next = validate_redirect_hop(&current, "https://config.example.com/b").unwrap();
        assert_eq!(next.as_str(), "https://config.example.com/b");
    }

    #[test]
    fn test_validate_redirect_hop_rejects_non_https_target() {
        let current = url::Url::parse("https://config.example.com/a").unwrap();
        let result = validate_redirect_hop(&current, "http://config.example.com/b");
        assert!(result.is_err(), "http redirect target must be rejected");
    }

    #[test]
    fn test_validate_redirect_hop_rejects_blocked_ip_targets() {
        let current = url::Url::parse("https://config.example.com/a").unwrap();
        // Redirects to private, loopback, link-local (cloud metadata) and
        // documentation IPs must be rejected.
        for location in [
            "https://127.0.0.1/admin",
            "https://10.0.0.1/internal",
            "https://169.254.169.254/latest/meta-data/",
            "https://192.0.2.1/doc",
            "https://[::1]/admin",
        ] {
            let result = validate_redirect_hop(&current, location);
            assert!(
                result.is_err(),
                "redirect to {location} must be rejected by SSRF checks"
            );
        }
    }

    #[test]
    fn test_validate_redirect_hop_invalid_location() {
        let current = url::Url::parse("https://config.example.com/a").unwrap();
        // "http://" has no host and cannot be resolved against the base URL.
        let result = validate_redirect_hop(&current, "http://");
        assert!(result.is_err(), "unparseable Location must be rejected");
    }

    // Helper for test assertions
    fn err_if_blocked(result: &Result<url::Url, ConfigError>) -> bool {
        if let Err(e) = result {
            matches!(e, ConfigError::InvalidValue { .. })
        } else {
            false
        }
    }

    // =============================================================================
    // Builder Tests
    // =============================================================================

    #[test]
    fn test_http_polled_source_builder() {
        let source = HttpPolledSourceBuilder::new()
            .url("https://example.com/config.json")
            .interval(Duration::from_secs(30))
            .build()
            .expect("builder should succeed with valid public URL");

        assert_eq!(source.poll_interval(), Some(Duration::from_secs(30)));
        assert_eq!(
            source.source_id().as_str(),
            "http:https://example.com/config.json"
        );
    }

    #[test]
    fn test_http_polled_source_builder_requires_url() {
        let result = HttpPolledSourceBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_default_poll_interval() {
        let source = HttpPolledSourceBuilder::new()
            .url("https://example.com/config.json")
            .build()
            .expect("builder should succeed with valid public URL");

        assert_eq!(source.poll_interval(), Some(DEFAULT_POLL_INTERVAL));
    }

    #[test]
    fn test_builder_pattern() {
        let source = HttpPolledSourceBuilder::new()
            .url("https://example.com/config.yaml")
            .interval(Duration::from_secs(30))
            .format(Format::Yaml)
            .timeout(Duration::from_secs(5))
            .allowed_domain("example.com")
            .allowed_domains(["cdn.example.com", "assets.example.com"])
            .build()
            .expect("builder should succeed with valid public URL and whitelist");

        assert_eq!(source.poll_interval(), Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_builder_rejects_blocked_ip() {
        let result = HttpPolledSourceBuilder::new()
            .url("https://192.168.1.1/config.json")
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.user_message();
        assert!(
            msg.contains("private") || msg.contains("SSRF") || msg.contains("blocked"),
            "Expected SSRF-related error message, got: {}",
            msg
        );
    }

    #[test]
    fn test_builder_rejects_loopback() {
        let result = HttpPolledSourceBuilder::new()
            .url("https://127.0.0.1/config.json")
            .build();

        assert!(result.is_err());

        let result = HttpPolledSourceBuilder::new()
            .url("https://[::1]/config.json")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_accepts_whitelisted_domain() {
        // Issue #348: build() no longer resolves DNS, so a whitelisted domain
        // builds without any network access; the whitelist is consumed on the
        // async poll path (validate_url_full skips DNS for these domains).
        let source = HttpPolledSourceBuilder::new()
            .url("https://whitelisted-internal.local/config.json")
            .allowed_domain("whitelisted-internal.local")
            .build()
            .expect("build() must not perform blocking DNS resolution");
        assert_eq!(source.allowed_domains.len(), 1);
        assert_eq!(source.allowed_domains[0], "whitelisted-internal.local");
        assert!(source.enforce_ssrf, "builder-built sources enforce SSRF");
    }

    #[test]
    fn test_builder_build_does_not_resolve_dns() {
        // An unresolvable domain builds fine: DNS validation (and its
        // blocked-IP checks) happens asynchronously on every poll request.
        let result = HttpPolledSourceBuilder::new()
            .url("https://nonexistent-host-for-tests.invalid/config.json")
            .build();
        assert!(
            result.is_ok(),
            "build() must not perform blocking DNS: {:?}",
            result.err()
        );
    }

    // =============================================================================
    // Real Local HTTP Interaction (HTTP 远程源真实本地交互)
    // =============================================================================

    /// Serve one HTTP/1.1 response with the given status, headers and body,
    /// then close the connection.
    async fn serve_one(
        listener: tokio::net::TcpListener,
        status: &'static str,
        headers: &'static str,
        body: &'static str,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut stream, _) = listener.accept().await.expect("accept connection");
        let mut buf = [0u8; 4096];
        // Read the request line + headers (ignore body for GET).
        let _n = stream.read(&mut buf).await.expect("read request");
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .await
            .expect("write response");
        stream.flush().await.expect("flush response");
    }

    /// Construct an `HttpPolledSource` pointing at a real local HTTP server.
    ///
    /// The builder intentionally blocks loopback URLs (SSRF protection), so the
    /// struct is constructed directly here to exercise the real HTTP fetch path
    /// (`do_poll` → reqwest → local TCP server) against a live local service.
    fn source_against_local(addr: std::net::SocketAddr) -> HttpPolledSource {
        source_against_local_with_cb(addr, CircuitBreaker::new())
    }

    /// Like [`source_against_local`] but with a caller-supplied circuit breaker
    /// so recovery tests can control the failure threshold and backoff.
    fn source_against_local_with_cb(
        addr: std::net::SocketAddr,
        circuit_breaker: CircuitBreaker,
    ) -> HttpPolledSource {
        source_against_local_full(
            addr,
            Client::builder().build().expect("client build"),
            circuit_breaker,
        )
    }

    /// Like [`source_against_local`] but with caller-supplied client and
    /// circuit breaker (e.g. a client with `redirect::Policy::none` for
    /// redirect-following tests).
    fn source_against_local_full(
        addr: std::net::SocketAddr,
        client: Client,
        circuit_breaker: CircuitBreaker,
    ) -> HttpPolledSource {
        let url = format!("http://{addr}/config.json");
        HttpPolledSource {
            url: url.clone().into(),
            interval: Duration::from_secs(1),
            client,
            format: None,
            allowed_domains: Vec::new().into(),
            enforce_ssrf: false,
            cached: RwLock::new(None),
            last_etag: ArcSwap::new(Arc::new(None)),
            last_modified: ArcSwap::new(Arc::new(None)),
            source_id: SourceId::new(format!("http:{url}")),
            circuit_breaker: std::sync::Mutex::new(circuit_breaker),
        }
    }

    /// Remote-fetch critical-path metrics: latency histogram on every fetch
    /// and an error counter on failures, through the optional backend.
    #[tokio::test]
    #[serial_test::serial]
    async fn test_remote_fetch_metrics_recorded_on_error_and_success() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        crate::metrics::clear_metrics_backend();
        let recorder = crate::metrics::test_support::RecordingBackend::installed();

        // Error path: a closed local port fails fast and must record both the
        // latency histogram and the error counter.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let closed_addr = listener.local_addr().expect("local addr");
        drop(listener); // close the port so the connection is refused

        let source = source_against_local(closed_addr);
        assert!(source.poll().await.is_err(), "closed port must fail");
        assert!(
            recorder.counter_count(crate::metrics::names::REMOTE_FETCH_ERRORS_TOTAL) >= 1,
            "fetch errors must be counted"
        );
        assert!(
            recorder.histogram_count(crate::metrics::names::REMOTE_FETCH_DURATION_SECONDS) >= 1,
            "fetch latency must be recorded"
        );

        // Success path: a live local HTTP server records latency without
        // counting an error.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let body = r#"{"ok":true}"#;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let _n = stream.read(&mut buf).await.expect("read");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
            stream.flush().await.expect("flush");
        });

        let source = source_against_local(addr);
        assert!(source.poll().await.is_ok(), "live server must answer");
        server.await.expect("server task");

        crate::metrics::clear_metrics_backend();
    }

    /// Real local HTTP interaction with explicit value assertions.
    #[cfg_attr(
        not(feature = "json"),
        ignore = "requires json feature to parse HTTP response body"
    )]
    #[tokio::test]
    async fn test_poll_live_local_http_asserts_values() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let body = r#"{"app":{"host":"localhost","port":8080}}"#;

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let _n = stream.read(&mut buf).await.expect("read");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
            stream.flush().await.expect("flush");
        });

        let source = source_against_local(addr);

        let value = source.poll().await.expect("poll should succeed");

        let app = value.inner.as_map().expect("top-level map");
        let host = app
            .get("app")
            .expect("app key")
            .inner
            .as_map()
            .expect("app map");
        assert_eq!(
            host.get("host").and_then(|v| v.as_str()),
            Some("localhost"),
            "host value from live HTTP response"
        );
        assert_eq!(
            host.get("port").and_then(|v| v.as_i64()),
            Some(8080),
            "port value from live HTTP response"
        );

        server.await.expect("server task");
    }

    /// Real local HTTP interaction: 304 Not Modified must return the cached
    /// value from the first successful poll.
    #[cfg_attr(
        not(feature = "json"),
        ignore = "requires json feature to parse HTTP response body"
    )]
    #[tokio::test]
    async fn test_poll_live_local_http_etag_cached_304() {
        if super::super::test_support::localhost_proxy_intercept().await {
            eprintln!("[skip] 检测到本机代理拦截 127.0.0.1 流量，localhost 网络断言不可靠");
            return;
        }
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let body = r#"{"app":{"host":"localhost","port":8080}}"#;

        let server = tokio::spawn(async move {
            // First request: 200 with ETag. Second request: 304 (cached).
            for (idx, _) in (0..2).enumerate() {
                let (mut stream, _) = listener.accept().await.expect("accept");
                let mut buf = [0u8; 4096];
                let _n = stream.read(&mut buf).await.expect("read");
                let (status, headers, payload) = if idx == 0 {
                    ("200 OK", "ETag: \"v1\"", body)
                } else {
                    ("304 Not Modified", "ETag: \"v1\"", "")
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                stream.write_all(response.as_bytes()).await.expect("write");
                stream.flush().await.expect("flush");
            }
        });

        let source = source_against_local(addr);

        let first = source.poll().await.expect("first poll succeeds");
        assert!(
            first.inner.as_map().expect("map").get("app").is_some(),
            "first poll returned config"
        );

        // Second poll sends If-None-Match; server replies 304 → cached value.
        let second = source.poll().await.expect("second poll succeeds");
        assert!(
            second.inner.as_map().expect("map").get("app").is_some(),
            "304 must return the cached config"
        );

        server.await.expect("server task");
    }

    /// Real local HTTP interaction: HTTP 500 must surface a RemoteUnavailable
    /// error (fail loud, Rule 12), not panic or return stale data.
    #[tokio::test]
    async fn test_poll_live_local_http_500_errors() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(serve_one(listener, "500 Internal Server Error", "", ""));

        let source = source_against_local(addr);

        let result = source.poll().await;
        assert!(
            result.is_err(),
            "HTTP 500 must surface a RemoteUnavailable error"
        );
        assert!(
            matches!(result.unwrap_err(), ConfigError::RemoteUnavailable { .. }),
            "expected RemoteUnavailable error"
        );

        server.await.expect("server task");
    }

    /// Integrated circuit-breaker recovery against a real local HTTP service
    /// (R-scenario-coverage: 远程源轮询失败熔断与恢复).
    ///
    /// Sequence: success → threshold failures (500s) → circuit opens and polls
    /// are skipped without touching the network → backoff elapses → HalfOpen
    /// probe succeeds → circuit closes and normal polls resume.
    #[cfg_attr(
        not(feature = "json"),
        ignore = "requires json feature to parse HTTP response body"
    )]
    #[tokio::test]
    async fn test_poll_live_local_http_circuit_breaker_open_and_recovers() {
        if super::super::test_support::localhost_proxy_intercept().await {
            eprintln!("[skip] 检测到本机代理拦截 127.0.0.1 流量，localhost 网络断言不可靠");
            return;
        }
        use std::sync::Arc as StdArc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");

        // Track how many HTTP requests the server actually receives so we can
        // prove that once the circuit is open, polls are skipped without
        // touching the network (no connection is accepted).
        let connections = StdArc::new(AtomicUsize::new(0));
        let conns = connections.clone();

        let server = tokio::spawn(async move {
            // Accept up to 6 connections: 1 success + 3 failures (threshold 3)
            // + 1 HalfOpen probe + 1 post-recovery poll. Any request beyond
            // that would mean the circuit failed to block polls while Open.
            for _ in 0..6 {
                let (mut stream, _) = listener.accept().await.expect("accept");
                conns.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 4096];
                let _n = stream.read(&mut buf).await.expect("read");
                // First request succeeds; failures until the circuit opens,
                // then any connection from the HalfOpen probe onward succeeds
                // again (recovery).
                let n = conns.load(Ordering::SeqCst);
                let (status, body) = if n == 1 || n >= 5 {
                    ("200 OK", r#"{"app":{"host":"localhost","port":8080}}"#)
                } else {
                    ("500 Internal Server Error", "")
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).await.expect("write");
                stream.flush().await.expect("flush");
            }
        });

        // Threshold 3 with a tiny base delay so the backoff elapses quickly.
        let cb = CircuitBreaker::new()
            .with_threshold(3)
            .with_base_delay(Duration::from_millis(50))
            .with_max_delay(Duration::from_millis(100));
        let source = source_against_local_with_cb(addr, cb);

        // 1. First poll succeeds and records a success (circuit stays Closed).
        let first = source.poll().await.expect("first poll succeeds");
        assert!(
            first.inner.as_map().expect("map").get("app").is_some(),
            "first poll returned config"
        );

        // 2. Three consecutive 500s push the circuit from Closed to Open.
        for _ in 0..3 {
            let result = source.poll().await;
            assert!(
                matches!(result, Err(ConfigError::RemoteUnavailable { .. })),
                "500 must surface RemoteUnavailable"
            );
        }
        assert_eq!(
            connections.load(Ordering::SeqCst),
            4,
            "exactly 4 HTTP requests made before the circuit opens"
        );

        // 3. While Open, polls are skipped entirely — no network traffic.
        let result = source.poll().await;
        assert!(
            matches!(
                result,
                Err(ConfigError::RemoteUnavailable { error_type, .. })
                    if error_type == "CircuitBreakerOpen"
            ),
            "open circuit must short-circuit the poll before any HTTP request"
        );
        assert_eq!(
            connections.load(Ordering::SeqCst),
            4,
            "no additional HTTP request while the circuit is open"
        );

        // 4. Wait for the backoff to elapse, then the HalfOpen probe is sent and
        //    the server replies 200 → the circuit recovers to Closed.
        tokio::time::sleep(Duration::from_millis(150)).await;
        let probe = source.poll().await.expect("HalfOpen probe succeeds");
        assert!(
            probe.inner.as_map().expect("map").get("app").is_some(),
            "recovered poll returned config"
        );

        // 5. A normal poll after recovery succeeds and is served by the real HTTP
        //    server (connection 6).
        let recovered = source.poll().await.expect("poll after recovery succeeds");
        assert!(
            recovered.inner.as_map().expect("map").get("app").is_some(),
            "normal poll after recovery returned config"
        );
        assert_eq!(
            connections.load(Ordering::SeqCst),
            6,
            "recovered circuit resumes making real HTTP requests"
        );

        server.await.expect("server task");
    }

    // =============================================================================
    // Manual Redirect Following Tests (Issue #347)
    // =============================================================================

    /// A reqwest client with automatic redirect following disabled, matching
    /// what `HttpPolledSourceBuilder::build()` configures.
    fn client_no_redirect() -> Client {
        Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("client build")
    }

    /// Issue #347: a 302 with a relative Location is followed manually and
    /// the final 200 response is parsed — proving the manual loop works where
    /// the client itself no longer follows redirects.
    #[cfg_attr(
        not(feature = "json"),
        ignore = "requires json feature to parse HTTP response body"
    )]
    #[tokio::test]
    async fn test_poll_follows_redirect_manually() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let body = r#"{"app":{"host":"redirected"}}"#;

        let server = tokio::spawn(async move {
            // Request 1: 302 with a relative Location. Request 2: the real
            // config at the redirect target.
            for i in 0..2 {
                let (mut stream, _) = listener.accept().await.expect("accept");
                let mut buf = [0u8; 4096];
                let _n = stream.read(&mut buf).await.expect("read");
                let (status, headers, payload) = if i == 0 {
                    ("302 Found", "Location: /real/config.json\r\n", "")
                } else {
                    ("200 OK", "", body)
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                    payload.len()
                );
                stream.write_all(response.as_bytes()).await.expect("write");
                stream.flush().await.expect("flush");
            }
        });

        let source = source_against_local_full(addr, client_no_redirect(), CircuitBreaker::new());

        let value = source
            .poll()
            .await
            .expect("poll must follow the redirect manually");

        let host = value
            .inner
            .as_map()
            .expect("map")
            .get("app")
            .expect("app key")
            .inner
            .as_map()
            .expect("app map")
            .get("host")
            .and_then(|v| v.as_str());
        assert_eq!(host, Some("redirected"), "config fetched after redirect");

        server.await.expect("server task");
    }

    /// Issue #347: redirect following is bounded — a redirect loop must stop
    /// after `MAX_REDIRECTS` hops with a non-retryable TooManyRedirects error
    /// instead of looping forever.
    #[tokio::test]
    async fn test_poll_redirect_loop_is_bounded() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");

        let server = tokio::spawn(async move {
            // The initial request + MAX_REDIRECTS (10) followed hops.
            for _ in 0..(MAX_REDIRECTS + 1) {
                let (mut stream, _) = listener.accept().await.expect("accept");
                let mut buf = [0u8; 4096];
                let _n = stream.read(&mut buf).await.expect("read");
                let response = "HTTP/1.1 302 Found\r\nContent-Type: application/json\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                    .to_string();
                stream.write_all(response.as_bytes()).await.expect("write");
                stream.flush().await.expect("flush");
            }
        });

        let source = source_against_local_full(addr, client_no_redirect(), CircuitBreaker::new());

        let result = source.poll().await;
        match result {
            Err(ConfigError::RemoteUnavailable {
                error_type,
                retryable: false,
            }) if error_type == "TooManyRedirects" => {}
            other => panic!("redirect loop must surface TooManyRedirects, got: {other:?}"),
        }

        server.await.expect("server task");
    }

    /// Issue #347: a 3xx response without a Location header is an error, not
    /// a silent stop.
    #[tokio::test]
    async fn test_poll_redirect_without_location_errors() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let _n = stream.read(&mut buf).await.expect("read");
            let response = "HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream.write_all(response.as_bytes()).await.expect("write");
            stream.flush().await.expect("flush");
        });

        let source = source_against_local_full(addr, client_no_redirect(), CircuitBreaker::new());

        let result = source.poll().await;
        match result {
            Err(ConfigError::RemoteUnavailable { error_type, .. })
                if error_type == "MissingRedirectLocation" => {}
            other => panic!("3xx without Location must error, got: {other:?}"),
        }

        server.await.expect("server task");
    }
}
