use crate::error::ConfigError;
use crate::providers::provider::{ConfigProvider, ProviderMetadata, ProviderType};
use figment::Figment;

pub struct HttpConfigProvider {
    url: String,
    username: Option<String>,
    password: Option<String>,
    bearer_token: Option<String>,
    ca_cert: Option<std::path::PathBuf>,
    client_cert: Option<std::path::PathBuf>,
    client_key: Option<std::path::PathBuf>,
    timeout: Option<String>,
    priority: u8,
}

impl HttpConfigProvider {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            username: None,
            password: None,
            bearer_token: None,
            ca_cert: None,
            client_cert: None,
            client_key: None,
            timeout: None,
            priority: 30, // 远程配置优先级较低
        }
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    pub fn with_tls(
        mut self,
        ca_cert: impl Into<std::path::PathBuf>,
        client_cert: Option<impl Into<std::path::PathBuf>>,
        client_key: Option<impl Into<std::path::PathBuf>>,
    ) -> Self {
        self.ca_cert = Some(ca_cert.into());
        self.client_cert = client_cert.map(|p| p.into());
        self.client_key = client_key.map(|p| p.into());
        self
    }

    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.timeout = Some(timeout.into());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    fn create_http_provider(&self) -> crate::providers::remote::http::HttpProvider {
        let mut provider = crate::providers::remote::http::HttpProvider::new(self.url.clone());

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            provider = provider.with_auth(username.clone(), password.clone());
        }

        if let Some(token) = &self.bearer_token {
            provider = provider.with_bearer_token(token.clone());
        }

        if let Some(timeout) = &self.timeout {
            provider = provider.with_timeout(timeout.clone());
        }

        if let Some(ca_cert) = &self.ca_cert {
            provider = provider.with_tls(
                ca_cert.clone(),
                self.client_cert.clone(),
                self.client_key.clone(),
            );
        }

        provider
    }
}

impl ConfigProvider for HttpConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let http_provider = self.create_http_provider();
        http_provider.load_sync()
    }

    fn name(&self) -> &str {
        "http"
    }

    fn is_available(&self) -> bool {
        // 检查URL是否有效
        !self.url.is_empty()
            && (self.url.starts_with("http://") || self.url.starts_with("https://"))
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        let auth_type = if self.bearer_token.is_some() {
            "bearer_token"
        } else if self.username.is_some() {
            "basic_auth"
        } else {
            "none"
        };

        ProviderMetadata {
            name: self.name().to_string(),
            description: format!("HTTP provider for URL: {} (auth: {})", self.url, auth_type),
            source_type: ProviderType::Remote,
            requires_network: true,
            supports_watch: false, // HTTP不支持原生watch
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
