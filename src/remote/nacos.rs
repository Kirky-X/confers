// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Nacos configuration source (`nacos` feature).
//!
//! Pulls a single `dataId` from a Nacos server through the HTTP Open API
//! (`GET /nacos/v1/cs/configs`) and listens for changes with a timed poll:
//! unchanged content short-circuits into the cached snapshot, so the parse
//! and merge work only run when the server actually served new content.
//!
//! Authentication: when `username`/`password` are configured, the
//! source logs in against `/nacos/v1/auth/login` and appends the returned
//! `accessToken` to every config request; an unauthorized (401) response
//! triggers one re-login and retry.
//!
//! Like every remote source this is an MVP adapter: content is projected
//! with the standard format pipeline (explicit format or content sniffing).

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;

use crate::error::{ConfigError, ConfigResult};
use crate::i18n::{tr, tr_args};
use crate::loader::Format;
use crate::remote::circuit_breaker::CircuitBreaker;
use crate::remote::common::try_parse_value_with_format;
use crate::types::{AnnotatedValue, SourceId};

const SOURCE_NAME: &str = "nacos";

/// Default Nacos group when none is configured.
pub const DEFAULT_NACOS_GROUP: &str = "DEFAULT_GROUP";

/// Builder for [`NacosSource`].
#[derive(Clone)]
pub struct NacosSourceBuilder {
    server: String,
    data_id: String,
    group: String,
    namespace: Option<String>,
    username: Option<String>,
    /// Never rendered by [`fmt::Debug`] (credentials must not leak through
    /// logs).
    password: Option<String>,
    format: Option<Format>,
    interval: Duration,
    timeout: Duration,
    cb_threshold: u32,
}

impl fmt::Debug for NacosSourceBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NacosSourceBuilder")
            .field("server", &self.server)
            .field("data_id", &self.data_id)
            .field("group", &self.group)
            .field("namespace", &self.namespace)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .field("format", &self.format)
            .field("interval", &self.interval)
            .field("timeout", &self.timeout)
            .field("cb_threshold", &self.cb_threshold)
            .finish()
    }
}

impl NacosSourceBuilder {
    /// Watch `dataId` on the Nacos `server` (e.g. `http://nacos:8848`).
    pub fn new(server: impl Into<String>, data_id: impl Into<String>) -> Self {
        Self {
            server: server.into().trim_end_matches('/').to_string(),
            data_id: data_id.into(),
            group: DEFAULT_NACOS_GROUP.to_string(),
            namespace: None,
            username: None,
            password: None,
            format: None,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            cb_threshold: 5,
        }
    }

    /// Set the group (default: `DEFAULT_GROUP`).
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// Set the namespace/tenant.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the username for Nacos authentication.
    ///
    /// Requires `password` to be set as well; with both configured the source
    /// logs in against `/nacos/v1/auth/login` and appends the returned
    /// `accessToken` to every config request.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the password for Nacos authentication.
    ///
    /// The value is redacted in `Debug` output and never appears in errors.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Pin a value format (default: content sniffing).
    pub fn format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Timed-listening poll interval (default: 30s).
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Per-request timeout (default: 10s).
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Circuit-breaker failure threshold before the source cools down.
    pub fn circuit_breaker_threshold(mut self, threshold: u32) -> Self {
        self.cb_threshold = threshold;
        self
    }

    /// Build the source.
    pub fn build(self) -> ConfigResult<NacosSource> {
        if self.server.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "server URL".to_string(),
                message: "nacos server must not be empty".to_string(),
            });
        }
        if self.data_id.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "dataId".to_string(),
                message: "nacos dataId must not be empty".to_string(),
            });
        }
        // auth requires both credentials or neither.
        match (&self.username, &self.password) {
            (Some(_), Some(_)) | (None, None) => {}
            (Some(_), None) => {
                return Err(ConfigError::InvalidValue {
                    key: "nacos.username".to_string(),
                    expected_type: "both username and password".to_string(),
                    message: tr("error-nacos-auth-password-missing"),
                });
            }
            (None, Some(_)) => {
                return Err(ConfigError::InvalidValue {
                    key: "nacos.password".to_string(),
                    expected_type: "both username and password".to_string(),
                    message: tr("error-nacos-auth-username-missing"),
                });
            }
        }
        let mut url = format!(
            "{}/nacos/v1/cs/configs?dataId={}&group={}",
            self.server,
            urlencode(&self.data_id),
            urlencode(&self.group)
        );
        if let Some(ref tenant) = self.namespace {
            url.push_str(&format!("&tenant={}", urlencode(tenant)));
        }
        let login_url = if self.username.is_some() {
            Some(format!("{}/nacos/v1/auth/login", self.server))
        } else {
            None
        };
        Ok(NacosSource {
            url: url.into(),
            login_url: login_url.map(Arc::from),
            username: self.username.map(Arc::from),
            password: self.password.map(Arc::from),
            access_token: ArcSwap::new(Arc::new(None)),
            format: self.format,
            interval: self.interval,
            client: reqwest::Client::builder()
                .timeout(self.timeout)
                .build()
                .map_err(|e| ConfigError::InvalidValue {
                    key: SOURCE_NAME.to_string(),
                    expected_type: "HTTP client".to_string(),
                    message: format!("failed to build nacos HTTP client: {e}"),
                })?,
            cached: ArcSwap::new(Arc::new(None)),
            last_content: ArcSwap::new(Arc::new(None)),
            source_id: SourceId::new(format!("{SOURCE_NAME}:{}:{}", self.group, self.data_id)),
            circuit_breaker: std::sync::Mutex::new(
                CircuitBreaker::new().with_threshold(self.cb_threshold),
            ),
        })
    }
}

/// Percent-encode a query parameter value (conservative form encoding).
fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Nacos HTTP Open API configuration source with timed change listening.
pub struct NacosSource {
    url: Arc<str>,
    /// Login endpoint (`/nacos/v1/auth/login`), present only when
    /// username/password auth is configured.
    login_url: Option<Arc<str>>,
    username: Option<Arc<str>>,
    /// Held, never logged and never rendered by `Debug`.
    password: Option<Arc<str>>,
    /// Cached `accessToken` from the last successful login.
    access_token: ArcSwap<Option<Arc<str>>>,
    format: Option<Format>,
    interval: Duration,
    client: reqwest::Client,
    cached: ArcSwap<Option<Arc<AnnotatedValue>>>,
    /// Raw content served on the previous successful poll (dedupe marker).
    last_content: ArcSwap<Option<Arc<str>>>,
    source_id: SourceId,
    circuit_breaker: std::sync::Mutex<CircuitBreaker>,
}

/// Outcome of the circuit-breaker gate at the top of `poll`.
enum Gate {
    /// A request may be attempted; its outcome is recorded.
    Record,
    /// A request may be attempted, but the breaker lock was contended —
    /// the outcome is NOT recorded (no failure counting, no spurious open).
    Unrecorded,
}

/// Internal classification of a failed config fetch: a 401 is
/// retried once after re-login, everything else propagates.
enum FetchFailure {
    Unauthorized(ConfigError),
    Other(ConfigError),
}

impl NacosSource {
    /// The Open API endpoint being polled.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Login and cache the returned `accessToken`.
    ///
    /// Nacos expects a form-encoded POST and answers with a JSON body
    /// containing `accessToken`. The password never appears in errors: only
    /// fixed messages are produced on failure.
    async fn login(&self) -> ConfigResult<()> {
        let Some(ref login_url) = self.login_url else {
            return Ok(());
        };
        let (Some(username), Some(password)) = (self.username.as_ref(), self.password.as_ref())
        else {
            return Ok(());
        };
        let form = format!(
            "username={}&password={}",
            urlencode(username),
            urlencode(password)
        );
        let response = self
            .client
            .post(login_url.as_ref())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .map_err(|_| ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "nacos auth response".to_string(),
                message: tr("error-nacos-login-request-failed"),
            })?;
        if !response.status().is_success() {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "2xx status".to_string(),
                message: tr_args(
                    "error-nacos-login-rejected",
                    &[("status", response.status().to_string())],
                ),
            });
        }
        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|_| ConfigError::InvalidValue {
                    key: SOURCE_NAME.to_string(),
                    expected_type: "nacos login JSON body".to_string(),
                    message: tr("error-nacos-login-body-unreadable"),
                })?;
        let token = body
            .get("accessToken")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "accessToken in login response".to_string(),
                message: tr("error-nacos-login-no-token"),
            })?;
        self.access_token
            .store(Arc::new(Some(Arc::from(token.to_string()))));
        Ok(())
    }

    /// Fetch the current content from the server.
    ///
    /// With auth configured the cached `accessToken` is appended; an
    /// unauthorized (401) response triggers exactly one re-login + retry
    async fn fetch(&self) -> ConfigResult<String> {
        if self.login_url.is_some() && self.access_token.load().is_none() {
            self.login().await?;
        }

        match self.fetch_once().await {
            Ok(content) => Ok(content),
            Err(FetchFailure::Unauthorized(err)) => {
                if self.login_url.is_some() {
                    // Token expired/invalid: re-login once and retry once.
                    self.login().await?;
                    return match self.fetch_once().await {
                        Ok(content) => Ok(content),
                        Err(FetchFailure::Unauthorized(err) | FetchFailure::Other(err)) => Err(err),
                    };
                }
                Err(err)
            }
            Err(FetchFailure::Other(err)) => Err(err),
        }
    }

    /// One config request (no auth orchestration).
    async fn fetch_once(&self) -> Result<String, FetchFailure> {
        let mut url = self.url.to_string();
        if let Some(token) = self.access_token.load().as_ref() {
            url.push_str(&format!("&accessToken={}", urlencode(token)));
        }
        let response = self.client.get(&url).send().await.map_err(|e| {
            // reqwest 的错误 Display 含完整 URL——而 accessToken 就
            // 挂在查询串上,原样格式化会把凭据泄进错误信息/日志。
            // without_url 原地剥离 URL(含 accessToken 查询串)。
            let message = tr_args(
                "error-nacos-request-failed",
                &[("message", e.without_url().to_string())],
            );
            FetchFailure::Other(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "nacos API response".to_string(),
                message,
            })
        })?;
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            // The URL is deliberately kept out of the unauthorized message:
            // access tokens travel in the query string.
            return Err(FetchFailure::Unauthorized(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "2xx status".to_string(),
                message: tr("error-nacos-unauthorized"),
            }));
        }
        if !status.is_success() {
            return Err(FetchFailure::Other(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "2xx status".to_string(),
                message: format!("nacos returned {status} for {}", self.url),
            }));
        }
        response.text().await.map_err(|e| {
            FetchFailure::Other(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "nacos config text".to_string(),
                message: format!("nacos returned an unreadable body: {e}"),
            })
        })
    }

    /// Project raw content into an annotated config value.
    fn project(&self, content: &str) -> AnnotatedValue {
        try_parse_value_with_format(content, self.format, SOURCE_NAME).unwrap_or_else(|| {
            AnnotatedValue::new(
                crate::types::ConfigValue::String(content.to_string()),
                SourceId::new(SOURCE_NAME),
                "",
            )
        })
    }
}

#[async_trait::async_trait]
impl crate::remote::PolledSource for NacosSource {
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        // Circuit breaker: a failing server cools the source down instead of
        // hammering it every interval. A CONTENDED breaker lock
        // (`try_lock` WouldBlock, e.g. another poll in flight) is recorded as
        // `unknown` — the request proceeds but its outcome is not recorded, so
        // concurrency no longer misreports the circuit as open.
        let gate = match self.circuit_breaker.try_lock() {
            Ok(mut cb) => {
                if cb.can_execute() {
                    Gate::Record
                } else {
                    // Genuinely open: fail fast without a request.
                    return Err(ConfigError::InvalidValue {
                        key: SOURCE_NAME.to_string(),
                        expected_type: "available source".to_string(),
                        message: tr("error-nacos-circuit-breaker-open"),
                    });
                }
            }
            // Lock contended: skip breaker accounting for this poll.
            Err(_) => Gate::Unrecorded,
        };

        // Remote-fetch critical-path metrics (manual: fetch resolves to raw
        // text, while record_fetch_metrics is bound to AnnotatedValue).
        let started = std::time::Instant::now();
        let labels = [("source", self.source_id.as_str())];
        let result = self.fetch().await;
        crate::metrics::record_histogram(
            crate::metrics::names::REMOTE_FETCH_DURATION_SECONDS,
            started.elapsed().as_secs_f64(),
            &labels,
        );
        let content = match result {
            Ok(content) => {
                if let Gate::Record = gate
                    && let Ok(mut cb) = self.circuit_breaker.try_lock()
                {
                    cb.record_success();
                }
                content
            }
            Err(err) => {
                crate::metrics::record_counter(
                    crate::metrics::names::REMOTE_FETCH_ERRORS_TOTAL,
                    &labels,
                );
                if let Gate::Record = gate
                    && let Ok(mut cb) = self.circuit_breaker.try_lock()
                {
                    cb.record_failure();
                }
                return Err(err);
            }
        };

        // Timed listening: unchanged content keeps serving the cached
        // snapshot (no re-parse, no provenance churn).
        let unchanged = {
            let last = self.last_content.load();
            last.as_deref() == Some(content.as_str())
        };
        if unchanged {
            let cached = self.cached.load();
            if let Some(ref value) = **cached {
                return Ok((**value).clone());
            }
        }

        let value = self.project(&content);
        self.last_content
            .store(Arc::new(Some(Arc::from(content.as_str()))));
        self.cached.store(Arc::new(Some(Arc::new(value.clone()))));
        Ok(value)
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(self.interval)
    }

    fn source_id(&self) -> SourceId {
        self.source_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock Nacos server: serves the queued bodies in order on
    /// `/nacos/v1/cs/configs`, counting how many requests arrived.
    async fn spawn_mock_nacos(bodies: Vec<String>) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");
        let hits = Arc::new(AtomicUsize::new(0));
        let hits_server = Arc::clone(&hits);
        let queue = Arc::new(std::sync::Mutex::new(bodies));

        tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let hits = Arc::clone(&hits_server);
                let queue = Arc::clone(&queue);
                tokio::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let _ = stream.read(&mut buf).await;
                    hits.fetch_add(1, Ordering::SeqCst);
                    let body = {
                        let mut q = queue.lock().unwrap();
                        if q.len() > 1 {
                            q.remove(0)
                        } else {
                            q.first().cloned().unwrap_or_default()
                        }
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
        (addr, hits)
    }

    fn source_for(addr: std::net::SocketAddr, group: &str) -> NacosSource {
        NacosSourceBuilder::new(format!("http://{addr}"), "app-config.toml")
            .group(group)
            .namespace("public")
            .interval(Duration::from_secs(5))
            .build()
            .expect("build nacos source")
    }

    #[tokio::test]
    async fn fetches_and_parses_config_from_open_api() {
        let body = "log_level = \"debug\"\nreplicas = 3\n";
        let (addr, _) = spawn_mock_nacos(vec![body.to_string()]).await;
        let source = source_for(addr, "DEFAULT_GROUP");

        let polled = crate::remote::PolledSource::poll(&source)
            .await
            .expect("poll nacos");
        let level = polled
            .inner
            .as_map()
            .and_then(|m| m.get("log_level"))
            .and_then(|v| v.as_str());
        assert_eq!(level, Some("debug"), "toml body parses into config keys");
    }

    #[tokio::test]
    async fn timed_listening_serves_cached_snapshot_for_unchanged_content() {
        let body = "log_level = \"info\"\n";
        let (addr, hits) = spawn_mock_nacos(vec![body.to_string()]).await;
        let source = source_for(addr, "DEFAULT_GROUP");
        let poll = crate::remote::PolledSource::poll(&source);

        let first = poll.await.expect("first poll");
        let second = crate::remote::PolledSource::poll(&source)
            .await
            .expect("second poll (unchanged)");
        assert_eq!(
            first
                .inner
                .as_map()
                .and_then(|m| m.get("log_level"))
                .and_then(|v| v.as_str()),
            second
                .inner
                .as_map()
                .and_then(|m| m.get("log_level"))
                .and_then(|v| v.as_str()),
            "unchanged content serves the cached snapshot"
        );
        assert!(hits.load(Ordering::SeqCst) >= 2, "server hit on every poll");
    }

    #[tokio::test]
    async fn published_change_is_picked_up_on_next_poll() {
        let (addr, _) = spawn_mock_nacos(vec![
            "log_level = \"info\"\n".to_string(),
            "log_level = \"trace\"\n".to_string(),
        ])
        .await;
        let source = source_for(addr, "DEFAULT_GROUP");

        let before = crate::remote::PolledSource::poll(&source)
            .await
            .expect("initial");
        let after = crate::remote::PolledSource::poll(&source)
            .await
            .expect("after publish");
        let a = before
            .inner
            .as_map()
            .and_then(|m| m.get("log_level"))
            .and_then(|v| v.as_str());
        let b = after
            .inner
            .as_map()
            .and_then(|m| m.get("log_level"))
            .and_then(|v| v.as_str());
        assert_eq!(a, Some("info"));
        assert_eq!(
            b,
            Some("trace"),
            "server-side publish must reach the next poll"
        );
    }

    #[tokio::test]
    async fn error_response_is_reported_and_counted() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        drop(listener); // connection refused

        let source = source_for(addr, "DEFAULT_GROUP");
        let result = crate::remote::PolledSource::poll(&source).await;
        assert!(result.is_err(), "unreachable server must fail the poll");
    }

    #[test]
    fn builder_urls_map_namespace_group_and_dataid() {
        let source = NacosSourceBuilder::new("http://nacos.internal:8848/", "app config.toml")
            .group("MY_GROUP")
            .namespace("prod tenant")
            .build()
            .expect("build");
        let url = source.url();
        assert!(
            url.contains("/nacos/v1/cs/configs?"),
            "open api path: {url}"
        );
        assert!(
            url.contains("dataId=app%20config.toml"),
            "dataId encoded: {url}"
        );
        assert!(url.contains("group=MY_GROUP"), "group: {url}");
        assert!(url.contains("tenant=prod%20tenant"), "tenant: {url}");
    }

    #[test]
    fn builder_rejects_empty_server_and_dataid() {
        assert!(NacosSourceBuilder::new("", "x").build().is_err());
        assert!(NacosSourceBuilder::new("http://nacos", "").build().is_err());
    }

    // ==================== auth & circuit-breaker fairness ====================

    /// Mock Nacos server with auth: answers `POST /nacos/v1/auth/login`
    /// with a JSON `accessToken`, and `GET /nacos/v1/cs/configs` only when
    /// the request carries the valid token (else 401). Returns the address
    /// plus counters for logins and (rejected) config requests.
    #[allow(clippy::type_complexity)]
    async fn spawn_mock_nacos_with_auth(
        username: &str,
        password: &str,
        token: &'static str,
    ) -> (
        std::net::SocketAddr,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
        Arc<AtomicUsize>,
    ) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().unwrap();
        let logins = Arc::new(AtomicUsize::new(0));
        let logins_server = Arc::clone(&logins);
        let unauthorized = Arc::new(AtomicUsize::new(0));
        let unauthorized_server = Arc::clone(&unauthorized);
        let authorized = Arc::new(AtomicUsize::new(0));
        let authorized_server = Arc::clone(&authorized);
        let (username, password) = (username.to_string(), password.to_string());

        tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let logins = Arc::clone(&logins_server);
                let unauthorized = Arc::clone(&unauthorized_server);
                let authorized = Arc::clone(&authorized_server);
                let (username, password, token) = (username.clone(), password.clone(), token);
                tokio::spawn(async move {
                    let mut buf = [0u8; 8192];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let first_line = request.lines().next().unwrap_or_default();
                    if first_line.starts_with("POST /nacos/v1/auth/login") {
                        // Validate the form credentials.
                        let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
                        if body.contains(&format!("username={username}"))
                            && body.contains(&format!("password={password}"))
                        {
                            logins.fetch_add(1, Ordering::SeqCst);
                            let body = format!(r#"{{"accessToken":"{token}"}}"#);
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                        } else {
                            let _ = stream
                                .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                                .await;
                        }
                    } else if first_line.starts_with("GET /nacos/v1/cs/configs") {
                        if first_line.contains(&format!("accessToken={token}")) {
                            authorized.fetch_add(1, Ordering::SeqCst);
                            let body = "log_level = \"debug\"\n";
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(response.as_bytes()).await;
                        } else {
                            unauthorized.fetch_add(1, Ordering::SeqCst);
                            let _ = stream
                                .write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                                .await;
                        }
                    } else {
                        let _ = stream
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                            .await;
                    }
                    let _ = stream.flush().await;
                });
            }
        });
        (addr, logins, unauthorized, authorized)
    }

    /// with username/password configured, the source logs in and the
    /// config request carries the returned accessToken.
    #[tokio::test]
    async fn auth_login_sets_token_and_config_request_succeeds() {
        let (addr, logins, unauthorized, authorized) =
            spawn_mock_nacos_with_auth("nacos", "secret", "tok-123").await; // pragma: allowlist secret
        let source = NacosSourceBuilder::new(format!("http://{addr}"), "app-config.toml")
            .username("nacos")
            .password("secret") // pragma: allowlist secret
            .build()
            .expect("build");

        let polled = crate::remote::PolledSource::poll(&source)
            .await
            .expect("poll with auth must succeed");
        assert_eq!(
            polled
                .inner
                .as_map()
                .and_then(|m| m.get("log_level"))
                .and_then(|v| v.as_str()),
            Some("debug")
        );
        assert_eq!(logins.load(Ordering::SeqCst), 1, "exactly one login");
        assert_eq!(
            authorized.load(Ordering::SeqCst),
            1,
            "config request carried the valid token"
        );
        assert_eq!(
            unauthorized.load(Ordering::SeqCst),
            0,
            "no request rejected"
        );
    }

    /// when the cached token is rejected with 401, the source re-logins
    /// exactly once and retries the config request successfully.
    #[tokio::test]
    async fn auth_relogins_once_on_401() {
        let (addr, logins, unauthorized, authorized) =
            spawn_mock_nacos_with_auth("nacos", "secret", "tok-123").await; // pragma: allowlist secret
        let source = NacosSourceBuilder::new(format!("http://{addr}"), "app-config.toml")
            .username("nacos")
            .password("secret") // pragma: allowlist secret
            .build()
            .expect("build");

        // Pre-seed a stale token so the first config request is rejected.
        source
            .access_token
            .store(Arc::new(Some(Arc::from("stale-token"))));

        let polled = crate::remote::PolledSource::poll(&source)
            .await
            .expect("poll must succeed after one re-login");
        assert!(
            polled
                .inner
                .as_map()
                .and_then(|m| m.get("log_level"))
                .is_some()
        );
        assert_eq!(
            unauthorized.load(Ordering::SeqCst),
            1,
            "the stale-token request was rejected once"
        );
        assert_eq!(
            logins.load(Ordering::SeqCst),
            1,
            "exactly one re-login after 401"
        );
        assert_eq!(
            authorized.load(Ordering::SeqCst),
            1,
            "the retried request carried the fresh token"
        );
    }

    /// builder Debug output redacts the password.
    #[test]
    fn builder_debug_redacts_password() {
        let builder = NacosSourceBuilder::new("http://nacos:8848", "app.toml")
            .username("nacos")
            .password("super-secret-password"); // pragma: allowlist secret
        let debug = format!("{builder:?}");
        assert!(!debug.contains("super-secret-password"));
        assert!(debug.contains("[REDACTED]"));
    }

    /// auth requires both credentials or neither.
    #[test]
    fn builder_rejects_partial_credentials() {
        assert!(
            NacosSourceBuilder::new("http://nacos:8848", "app.toml")
                .username("nacos")
                .build()
                .is_err(),
            "username without password must fail the build"
        );
        assert!(
            NacosSourceBuilder::new("http://nacos:8848", "app.toml")
                .password("secret") // pragma: allowlist secret
                .build()
                .is_err(),
            "password without username must fail the build"
        );
    }

    /// a contended circuit-breaker lock (`try_lock` WouldBlock, e.g.
    /// another poll in flight) is recorded as unknown — the poll proceeds
    /// and must NOT be misreported as "circuit breaker is open".
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // 锁跨 await 正是被测场景
    async fn contended_circuit_breaker_does_not_report_open() {
        let body = "log_level = \"info\"\n".to_string();
        let (addr, _) = spawn_mock_nacos(vec![body]).await;
        let source = source_for(addr, "DEFAULT_GROUP");

        // Hold the breaker lock to simulate a concurrent poll in flight.
        let guard = source.circuit_breaker.try_lock().expect("hold lock");

        let result = crate::remote::PolledSource::poll(&source).await;
        drop(guard);
        assert!(
            result.is_ok(),
            "a contended breaker lock must not fail the poll: {:?}",
            result.err()
        );

        // Nothing was recorded while the lock was contended.
        assert_eq!(
            source.circuit_breaker.lock().unwrap().failure_count(),
            0,
            "no failure may be recorded for a contended round"
        );
    }
}
