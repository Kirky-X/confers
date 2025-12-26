use etcd_client::{Client, ConnectOptions, Identity, TlsOptions};
use figment::{
    value::{Dict, Map},
    Error, Profile, Provider,
};
// use rustls_pki_types::{CertificateDer, PrivateKeyDer}; // Not used directly, etcd-client handles PEM parsing
use failsafe::{backoff, failure_policy, CircuitBreaker, Config, Error as FailsafeError};
use std::fs;
use std::time::Duration;

pub struct EtcdProvider {
    endpoints: Vec<String>,
    key: String,
    username: Option<String>,
    password: Option<String>,
    ca_path: Option<String>,
    cert_path: Option<String>,
    key_path: Option<String>,
}

impl EtcdProvider {
    pub fn new(endpoints: Vec<String>, key: impl Into<String>) -> Self {
        Self {
            endpoints,
            key: key.into(),
            username: None,
            password: None,
            ca_path: None,
            cert_path: None,
            key_path: None,
        }
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    pub fn with_tls(
        mut self,
        ca_path: Option<String>,
        cert_path: Option<String>,
        key_path: Option<String>,
    ) -> Self {
        self.ca_path = ca_path;
        self.cert_path = cert_path;
        self.key_path = key_path;
        self
    }
}

impl Provider for EtcdProvider {
    fn metadata(&self) -> figment::Metadata {
        figment::Metadata::named(format!("Etcd ({:?})", self.endpoints))
    }

    fn data(&self) -> Result<Map<Profile, Dict>, Error> {
        // Clone data for the thread
        let endpoints = self.endpoints.clone();
        let key = self.key.clone();
        let username = self.username.clone();
        let password = self.password.clone();
        let ca_path = self.ca_path.clone();
        let cert_path = self.cert_path.clone();
        let key_path = self.key_path.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| Error::from(e.to_string()))?;

            // Circuit breaker configuration
            let circuit_breaker = Config::new()
                .failure_policy(failure_policy::consecutive_failures(
                    3,
                    backoff::constant(Duration::from_secs(10)),
                ))
                .build();

            let result = circuit_breaker.call(|| {
                rt.block_on(async {
                    let mut options = ConnectOptions::new();

                    if let (Some(u), Some(p)) = (&username, &password) {
                        options = options.with_user(u, p);
                    }

                    if let Some(ca) = &ca_path {
                        let ca_pem = fs::read_to_string(ca)
                            .map_err(|e| Error::from(format!("Failed to read CA file: {}", e)))?;
                        let mut tls = TlsOptions::new()
                            .ca_certificate(etcd_client::Certificate::from_pem(ca_pem));

                        if let (Some(cert), Some(key_p)) = (&cert_path, &key_path) {
                            let cert_pem = fs::read_to_string(cert).map_err(|e| {
                                Error::from(format!("Failed to read cert file: {}", e))
                            })?;
                            let key_pem = fs::read_to_string(key_p).map_err(|e| {
                                Error::from(format!("Failed to read key file: {}", e))
                            })?;

                            tls = tls.identity(Identity::from_pem(cert_pem, key_pem));
                        }

                        options = options.with_tls(tls);
                    }

                    let mut client = Client::connect(&endpoints, Some(options))
                        .await
                        .map_err(|e| Error::from(format!("Failed to connect to Etcd: {}", e)))?;

                    let resp = client
                        .get(key.clone(), None)
                        .await
                        .map_err(|e| Error::from(format!("Failed to get key from Etcd: {}", e)))?;

                    if let Some(kv) = resp.kvs().first() {
                        let val_str = kv.value_str().map_err(|e| Error::from(e.to_string()))?;
                        // Assume JSON content
                        let map: Dict = serde_json::from_str(val_str)
                            .map_err(|e| Error::from(format!("Failed to parse JSON: {}", e)))?;

                        let mut profiles = Map::new();
                        profiles.insert(Profile::Default, map);
                        Ok(profiles)
                    } else {
                        Err(Error::from(format!("Key {} not found in Etcd", key)))
                    }
                })
            });

            match result {
                Ok(res) => Ok(res),
                Err(FailsafeError::Inner(e)) => Err(e),
                Err(FailsafeError::Rejected) => {
                    Err(Error::from("Circuit breaker open: Etcd requests rejected"))
                }
            }
        })
        .join()
        .map_err(|_| Error::from("Etcd thread panicked"))?
    }
}
