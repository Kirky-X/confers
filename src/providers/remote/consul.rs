// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use failsafe::{
    backoff, failure_policy, CircuitBreaker, Config as CircuitBreakerConfig, Error as FailsafeError,
};
use figment::{
    value::{Dict, Map},
    Error, Profile, Provider,
};
use std::time::Duration;
use url::Url;

pub struct ConsulProvider {
    address: String,
    key: String,
    token: Option<String>,
}

impl ConsulProvider {
    pub fn new(address: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            key: key.into(),
            token: None,
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn with_auth(self, _username: impl Into<String>, _password: impl Into<String>) -> Self {
        // Consul primarily uses tokens. Basic auth is rarely used or configured differently.
        // We implement this no-op or partial support to satisfy the common remote interface.
        // If needed, we can store these and use them for basic auth header.
        self
    }
}

#[derive(serde::Deserialize)]
#[allow(non_snake_case)]
struct ConsulKvPair {
    Value: String,
}

impl Provider for ConsulProvider {
    fn metadata(&self) -> figment::Metadata {
        figment::Metadata::named(format!("Consul ({})", self.address))
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        let address = self.address.clone();
        let key = self.key.clone();
        let token = self.token.clone();

        let circuit_breaker = CircuitBreakerConfig::new()
            .failure_policy(failure_policy::consecutive_failures(
                3,
                backoff::constant(Duration::from_secs(10)),
            ))
            .build();

        let result = circuit_breaker.call(|| {
            let mut url = Url::parse(&address)
                .map_err(|e| Error::from(format!("Invalid Consul URL: {}", e)))?;

            // Normalize path logic to match standard Consul API /v1/kv/{key}
            // If user provides "http://localhost:8500", we want "http://localhost:8500/v1/kv/{key}"
            let path = url.path();
            if path == "/" || path.is_empty() {
                url.set_path(&format!("/v1/kv/{}", key));
            } else if path.ends_with("/v1/kv/") {
                url.set_path(&format!("{}{}", path, key));
            } else if path.contains("/v1/kv") {
                // Assume path ends with something that needs key appended
                let new_path = format!("{}/{}", path.trim_end_matches('/'), key);
                url.set_path(&new_path);
            } else {
                // No v1/kv in path, assume it's base URL
                let new_path = format!("{}/v1/kv/{}", path.trim_end_matches('/'), key);
                url.set_path(&new_path);
            }

            let client = reqwest::blocking::Client::new();
            let mut req = client.get(url.clone());

            if let Some(t) = &token {
                req = req.header("X-Consul-Token", t);
            }

            let resp = req
                .send()
                .map_err(|e| Error::from(format!("Failed to connect to Consul: {}", e)))?;

            if resp.status().is_success() {
                let kvs: Vec<ConsulKvPair> = resp
                    .json()
                    .map_err(|e| Error::from(format!("Failed to parse Consul response: {}", e)))?;

                if let Some(kv) = kvs.first() {
                    let val_str = &kv.Value;
                    // Decode Base64
                    let decoded = BASE64
                        .decode(val_str)
                        .map_err(|e| Error::from(format!("Base64 decode failed: {}", e)))?;

                    let json_str = String::from_utf8(decoded)
                        .map_err(|e| Error::from(format!("UTF-8 error: {}", e)))?;

                    let map: Dict = serde_json::from_str(&json_str)
                        .map_err(|e| Error::from(format!("Failed to parse JSON: {}", e)))?;

                    let mut profiles = Map::new();
                    profiles.insert(Profile::Default, map);
                    Ok(profiles)
                } else {
                    // Empty list implies key not found or empty
                    Err(Error::from(format!(
                        "Key {} not found in Consul (empty response)",
                        key
                    )))
                }
            } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
                Err(Error::from(format!("Key {} not found in Consul", key)))
            } else {
                Err(Error::from(format!(
                    "Consul returned error: {}",
                    resp.status()
                )))
            }
        });

        match result {
            Ok(res) => Ok(res),
            Err(FailsafeError::Inner(e)) => Err(e),
            Err(FailsafeError::Rejected) => Err(Error::from(
                "Circuit breaker open: Consul requests rejected",
            )),
        }
    }
}
