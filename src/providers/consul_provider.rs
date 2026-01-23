// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::provider::{ConfigProvider, ProviderMetadata, ProviderType};
#[cfg(feature = "encryption")]
use crate::security::{SecureString, SensitivityLevel};
use crate::utils::ssrf::validate_remote_url;
use crate::utils::tls_config::TlsConfig;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use failsafe::{
    backoff, failure_policy, CircuitBreaker, Config as CircuitBreakerConfig, Error as FailsafeError,
};
use figment::{
    providers::Serialized,
    value::{Dict, Map},
    Figment, Profile,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// ACL token type for Consul (requires encryption feature)
#[cfg(feature = "encryption")]
#[derive(Clone, Debug)]
pub enum ConsulAclToken {
    /// Standard ACL token
    Token(Arc<SecureString>),
    /// Bearer token format (common in Consul Enterprise)
    Bearer(Arc<SecureString>),
    /// Identity token for OIDC/JWT authentication
    Identity(Arc<SecureString>),
    /// Agent token (for agent-specific operations)
    Agent(Arc<SecureString>),
}

#[cfg(feature = "encryption")]
impl ConsulAclToken {
    /// Create a standard ACL token
    pub fn token(token: impl Into<String>) -> Self {
        ConsulAclToken::Token(Arc::new(SecureString::new(
            token.into(),
            SensitivityLevel::High,
        )))
    }

    /// Create a bearer token (for Consul Enterprise)
    pub fn bearer(token: impl Into<String>) -> Self {
        ConsulAclToken::Bearer(Arc::new(SecureString::new(
            token.into(),
            SensitivityLevel::High,
        )))
    }

    /// Create an identity token for OIDC/JWT
    pub fn identity(token: impl Into<String>) -> Self {
        ConsulAclToken::Identity(Arc::new(SecureString::new(
            token.into(),
            SensitivityLevel::High,
        )))
    }

    /// Create an agent token
    pub fn agent(token: impl Into<String>) -> Self {
        ConsulAclToken::Agent(Arc::new(SecureString::new(
            token.into(),
            SensitivityLevel::High,
        )))
    }

    fn as_str(&self) -> &str {
        match self {
            ConsulAclToken::Token(t) | ConsulAclToken::Bearer(t)
            | ConsulAclToken::Identity(t) | ConsulAclToken::Agent(t) => t.as_str(),
        }
    }

    fn header_name(&self) -> &'static str {
        match self {
            ConsulAclToken::Token(_) => "X-Consul-Token",
            ConsulAclToken::Bearer(_) => "Authorization",
            ConsulAclToken::Identity(_) => "X-Consul-Identity",
            ConsulAclToken::Agent(_) => "X-Consul-Agent-Token",
        }
    }
}

/// Simple ACL token type (without encryption feature)
#[cfg(not(feature = "encryption"))]
#[derive(Clone, Debug)]
pub enum ConsulAclToken {
    Token(String),
    Bearer(String),
    Identity(String),
    Agent(String),
}

#[cfg(not(feature = "encryption"))]
impl ConsulAclToken {
    pub fn token(token: impl Into<String>) -> Self {
        ConsulAclToken::Token(token.into())
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        ConsulAclToken::Bearer(token.into())
    }

    pub fn identity(token: impl Into<String>) -> Self {
        ConsulAclToken::Identity(token.into())
    }

    pub fn agent(token: impl Into<String>) -> Self {
        ConsulAclToken::Agent(token.into())
    }

    fn as_str(&self) -> &str {
        match self {
            ConsulAclToken::Token(t) | ConsulAclToken::Bearer(t)
            | ConsulAclToken::Identity(t) | ConsulAclToken::Agent(t) => t.as_str(),
        }
    }

    fn header_name(&self) -> &'static str {
        match self {
            ConsulAclToken::Token(_) => "X-Consul-Token",
            ConsulAclToken::Bearer(_) => "Authorization",
            ConsulAclToken::Identity(_) => "X-Consul-Identity",
            ConsulAclToken::Agent(_) => "X-Consul-Agent-Token",
        }
    }
}

/// Consul ACL policy configuration
#[derive(Clone, Default)]
pub struct ConsulAclPolicy {
    /// Policy ID or name
    pub policy_id: Option<String>,
    /// Role ID or name (Consul Enterprise)
    pub role_id: Option<String>,
    /// Namespace (Consul Enterprise)
    pub namespace: Option<String>,
    /// Partition (Consul Enterprise)
    pub partition: Option<String>,
    /// Datacenter
    pub datacenter: Option<String>,
}

impl ConsulAclPolicy {
    /// Create a new ACL policy
    pub fn new() -> Self {
        Self::default()
    }

    /// Set policy ID
    pub fn with_policy_id(mut self, policy_id: impl Into<String>) -> Self {
        self.policy_id = Some(policy_id.into());
        self
    }

    /// Set role ID (Consul Enterprise)
    pub fn with_role_id(mut self, role_id: impl Into<String>) -> Self {
        self.role_id = Some(role_id.into());
        self
    }

    /// Set namespace (Consul Enterprise)
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set partition (Consul Enterprise)
    pub fn with_partition(mut self, partition: impl Into<String>) -> Self {
        self.partition = Some(partition.into());
        self
    }

    /// Set datacenter
    pub fn with_datacenter(mut self, datacenter: impl Into<String>) -> Self {
        self.datacenter = Some(datacenter.into());
        self
    }
}

#[derive(Clone)]
pub struct ConsulConfigProvider {
    address: String,
    key: String,
    token: Option<ConsulAclToken>,
    acl_policy: Option<ConsulAclPolicy>,
    tls_config: Option<TlsConfig>,
    priority: u8,
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct ConsulKvPair {
    Value: String,
}

impl ConsulConfigProvider {
    pub fn new(address: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            key: key.into(),
            token: None,
            acl_policy: None,
            tls_config: None,
            priority: 30,
        }
    }

    pub fn from_address(address: impl Into<String>, key: impl Into<String>) -> Self {
        Self::new(address, key)
    }

    /// Set ACL token (standard Consul token)
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(ConsulAclToken::token(token));
        self
    }

    /// Set ACL token securely (encryption feature only)
    #[cfg(feature = "encryption")]
    pub fn with_token_secure(mut self, token: Arc<SecureString>) -> Self {
        self.token = Some(ConsulAclToken::Token(token));
        self
    }

    /// Set bearer token (for Consul Enterprise)
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(ConsulAclToken::bearer(token));
        self
    }

    /// Set identity token (for OIDC/JWT authentication)
    pub fn with_identity_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(ConsulAclToken::identity(token));
        self
    }

    /// Set agent token
    pub fn with_agent_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(ConsulAclToken::agent(token));
        self
    }

    /// Set ACL policy configuration
    pub fn with_acl_policy(mut self, policy: ConsulAclPolicy) -> Self {
        self.acl_policy = Some(policy);
        self
    }

    /// Set TLS configuration
    pub fn with_tls(
        mut self,
        ca_cert: impl Into<PathBuf>,
        client_cert: Option<impl Into<PathBuf>>,
        client_key: Option<impl Into<PathBuf>>,
    ) -> Self {
        self.tls_config = Some(TlsConfig {
            ca_cert: Some(ca_cert.into()),
            client_cert: client_cert.map(|p| p.into()),
            client_key: client_key.map(|p| p.into()),
        });
        self
    }

    /// Set priority (lower values are loaded first)
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Backward compatibility: with_auth does nothing (deprecated)
    pub fn with_auth(self, _username: impl Into<String>, _password: impl Into<String>) -> Self {
        self
    }

    fn build_client(&self) -> Result<reqwest::blocking::Client, ConfigError> {
        let mut client_builder = reqwest::blocking::Client::builder();

        if let Some(tls) = &self.tls_config {
            if let Some(ca_path) = &tls.ca_cert {
                let cert_data = std::fs::read(ca_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read CA cert: {}", e))
                })?;
                let cert = reqwest::Certificate::from_pem(&cert_data).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse CA cert: {}", e))
                })?;
                client_builder = client_builder.add_root_certificate(cert);
            }

            if let (Some(cert_path), Some(key_path)) = (&tls.client_cert, &tls.client_key) {
                let cert_data = std::fs::read(cert_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client cert: {}", e))
                })?;
                let key_data = std::fs::read(key_path).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to read client key: {}", e))
                })?;
                // Combine cert and key for identity
                let mut combined = cert_data;
                combined.extend_from_slice(b"\n");
                combined.extend_from_slice(&key_data);
                let identity = reqwest::Identity::from_pem(&combined).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse client identity: {}", e))
                })?;
                client_builder = client_builder.identity(identity);
            }
        }

        client_builder
            .build()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to build client: {}", e)))
    }

    fn build_url(&self) -> Result<Url, ConfigError> {
        let mut url = Url::parse(&self.address)
            .map_err(|e| ConfigError::RemoteError(format!("Invalid Consul URL: {}", e)))?;

        let path = url.path();
        let key = &self.key;

        // Build base path with key
        let base_path = if path == "/" || path.is_empty() {
            format!("/v1/kv/{}", key)
        } else if path.ends_with("/v1/kv/") {
            format!("{}{}", path, key)
        } else if path.contains("/v1/kv") {
            format!("{}/{}", path.trim_end_matches('/'), key)
        } else {
            format!("{}/v1/kv/{}", path.trim_end_matches('/'), key)
        };

        url.set_path(&base_path);

        // Add query parameters for ACL policy
        let mut query_pairs: Vec<(String, String)> = Vec::new();

        if let Some(policy) = &self.acl_policy {
            if let Some(ns) = &policy.namespace {
                query_pairs.push(("ns".to_string(), ns.clone()));
            }
            if let Some(partition) = &policy.partition {
                query_pairs.push(("partition".to_string(), partition.clone()));
            }
            if let Some(dc) = &policy.datacenter {
                query_pairs.push(("dc".to_string(), dc.clone()));
            }
            if let Some(pid) = &policy.policy_id {
                query_pairs.push(("policy".to_string(), pid.clone()));
            }
            if let Some(rid) = &policy.role_id {
                query_pairs.push(("role".to_string(), rid.clone()));
            }
        }

        // Only set query if we have parameters
        if !query_pairs.is_empty() {
            let query: String = query_pairs
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
            url.set_query(Some(&query));
        }

        Ok(url)
    }

    fn fetch_data(&self) -> Result<Map<Profile, Dict>, ConfigError> {
        let url = self.build_url()?;
        let client = self.build_client()?;

        let mut req = client.get(url);

        // Add ACL headers based on token type
        if let Some(token) = &self.token {
            req = req.header(token.header_name(), token.as_str());
        }

        let resp = req
            .send()
            .map_err(|e| ConfigError::RemoteError(format!("Failed to connect to Consul: {}", e)))?;

        if resp.status().is_success() {
            let kvs: Vec<ConsulKvPair> = resp.json().map_err(|e| {
                ConfigError::RemoteError(format!("Failed to parse Consul response: {}", e))
            })?;

            if let Some(kv) = kvs.first() {
                let val_str = &kv.Value;
                let decoded = BASE64.decode(val_str).map_err(|e| {
                    ConfigError::RemoteError(format!("Base64 decode failed: {}", e))
                })?;

                let json_str = String::from_utf8(decoded)
                    .map_err(|e| ConfigError::RemoteError(format!("UTF-8 error: {}", e)))?;

                let map: Dict = serde_json::from_str(&json_str).map_err(|e| {
                    ConfigError::RemoteError(format!("Failed to parse JSON: {}", e))
                })?;

                let mut profiles = Map::new();
                profiles.insert(Profile::Default, map);
                Ok(profiles)
            } else {
                Err(ConfigError::RemoteError(format!(
                    "Key {} not found in Consul (empty response)",
                    self.key
                )))
            }
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Err(ConfigError::RemoteError(format!(
                "Key {} not found in Consul",
                self.key
            )))
        } else if resp.status() == reqwest::StatusCode::FORBIDDEN {
            Err(ConfigError::RemoteError(
                "Access denied: ACL token insufficient permissions or invalid".to_string(),
            ))
        } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            Err(ConfigError::RemoteError(
                "Authentication failed: Invalid or expired ACL token".to_string(),
            ))
        } else {
            Err(ConfigError::RemoteError(format!(
                "Consul returned error: {}",
                resp.status()
            )))
        }
    }
}

impl ConfigProvider for ConsulConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        // Validate URL to prevent SSRF attacks
        validate_remote_url(&self.address)?;

        let circuit_breaker = CircuitBreakerConfig::new()
            .failure_policy(failure_policy::consecutive_failures(
                3,
                backoff::constant(Duration::from_secs(10)),
            ))
            .build();

        let result = circuit_breaker.call(|| self.fetch_data());

        match result {
            Ok(data) => {
                let figment = Figment::new().merge(Serialized::from(data, Profile::Default));
                Ok(figment)
            }
            Err(FailsafeError::Inner(e)) => Err(e),
            Err(FailsafeError::Rejected) => Err(ConfigError::RemoteError(
                "Circuit breaker open: Consul requests rejected".to_string(),
            )),
        }
    }

    fn name(&self) -> &str {
        "consul"
    }

    fn is_available(&self) -> bool {
        !self.address.is_empty() && self.address.starts_with("http")
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name().to_string(),
            description: format!("Consul provider for key: {}", self.key),
            source_type: ProviderType::Remote,
            requires_network: true,
            supports_watch: false,
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[deprecated(since = "0.4.0", note = "Use ConsulConfigProvider instead")]
pub type ConsulProvider = ConsulConfigProvider;