//! E2E gap scenarios: security rules, config injector, audit chain, encryption.

use base64::Engine as _;
use confers::security::rules::{
    CorsValidator, SecurityValidator, SsrfValidator, TlsConfigValidator,
};
use confers::{PathValidator, ip_blocklist, sensitive_names};

use confers::interface::ConfigProvider;
use confers::types::{AnnotatedValue, ConfigValue};

/// Minimal ConfigProvider over a key→ConfigValue map for validator tests.
struct MapProvider(Vec<(String, AnnotatedValue)>);

impl MapProvider {
    fn new() -> Self {
        MapProvider(vec![])
    }
    fn with(mut self, k: &str, v: ConfigValue) -> Self {
        self.0.push((k.to_string(), annotated(v)));
        self
    }
}

impl ConfigProvider for MapProvider {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
        // support nested lookup like "cors.origins" only if stored flat; keep flat
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    fn keys(&self) -> Vec<String> {
        self.0.iter().map(|(k, _)| k.clone()).collect()
    }
}

fn annotated(v: ConfigValue) -> AnnotatedValue {
    AnnotatedValue {
        inner: v,
        ..Default::default()
    }
}

// --- 140: URL boundary matching, no prefix-bypass; 0.0.0.0/8 in blocklist ---
#[test]
fn gs140_ssrf_url_boundary_no_prefix_bypass() {
    // whitelist entry must not prefix-match a lookalike domain
    let validator = SsrfValidator::with_config_key("server.url")
        .with_whitelist(vec!["https://127.0.0.1".to_string()]);
    let config = MapProvider::new().with(
        "server.url",
        ConfigValue::from("http://127.0.0.1.evil.com"), // prefix lookalike, and plain http
    );
    let res = SsrfValidator::validate(&validator, &config);
    let violations = match res {
        Ok(()) => panic!("lookalike domain must not be whitelisted"),
        Err(v) => v,
    };
    assert!(
        violations.iter().any(|v| v.message.contains("non-HTTPS")),
        "lookalike must not inherit whitelist exemption: {violations:?}"
    );
    // 0.0.0.0/8 present in the blocklist (rc.6 fix)
    assert!(ip_blocklist::is_ip_blocked(std::net::IpAddr::V4(
        std::net::Ipv4Addr::new(0, 0, 0, 0)
    )));
    assert!(ip_blocklist::is_ip_blocked(std::net::IpAddr::V4(
        std::net::Ipv4Addr::new(0, 255, 255, 255)
    )));
}

// --- 141: TLS version compared as integer tuples, "1.12" is not < "1.2" ---
#[test]
fn gs141_tls_version_tuple_comparison() {
    // 1.12 >= 1.2 numerically → must NOT be flagged
    let ok = MapProvider::new().with("tls.min_version", ConfigValue::from("1.12"));
    let res = SecurityValidator::validate(&TlsConfigValidator::new(), &ok);
    assert!(res.is_ok(), "1.12 must not be flagged below 1.2: {res:?}");
    // 1.0 < 1.2 → flagged
    let bad = MapProvider::new().with("tls.min_version", ConfigValue::from("1.0"));
    let res = SecurityValidator::validate(&TlsConfigValidator::new(), &bad);
    assert!(res.is_err(), "1.0 must be flagged");
}

// --- 143: negative cors.max_age does not wrap around u64 ---
#[test]
fn gs143_cors_negative_max_age_no_wraparound() {
    let config = MapProvider::new()
        .with("cors.origins", ConfigValue::from("https://a.com"))
        .with("cors.max_age", ConfigValue::I64(-5));
    let res = SecurityValidator::validate(&CorsValidator::new(), &config);
    match res {
        Ok(()) => { /* guard skipped the negative value: no wraparound false positive */ }
        Err(v) => {
            for viol in &v {
                let msg = &viol.message;
                assert!(
                    !msg.contains("18446744073709551611"),
                    "negative max_age must not wrap to huge u64: {msg}"
                );
            }
        }
    }
}

// --- 144: malformed IPv6 (missing bracket) reported, not silently skipped ---
#[test]
fn gs144_ipv6_missing_bracket_reported() {
    let validator = SsrfValidator::with_config_key("server.url");
    let config = MapProvider::new().with(
        "server.url",
        ConfigValue::from("https://[::1"), // missing closing bracket
    );
    let res = SecurityValidator::validate(&validator, &config);
    match res {
        Ok(()) => panic!("malformed IPv6 must not pass silently"),
        Err(v) => {
            eprintln!("gs144 violations = {v:?}");
            assert!(!v.is_empty(), "malformed IPv6 must produce a violation");
        }
    }
}

// --- 145: sensitive_names + PathValidator public surface ---
#[test]
fn gs145_sensitive_names_and_path_validator() {
    assert!(sensitive_names::is_sensitive_name("password"));
    assert!(sensitive_names::is_sensitive_name("api_key"));
    assert!(sensitive_names::is_sensitive_name("secret"));
    assert!(sensitive_names::is_sensitive_name("token"));
    assert!(!sensitive_names::is_sensitive_name("host"));
    // PathValidator rejects traversal and accepts a clean relative path
    let pv = PathValidator {};
    assert!(pv.validate_and_resolve("../../etc/passwd").is_err());
    // a clean relative path that does not exist must surface FileNotFound —
    // never a traversal/security error — proving no false-positive blocking
    match pv.validate_and_resolve("config/app.toml") {
        Ok(p) => eprintln!("gs145 resolved = {p:?}"),
        Err(e) => {
            let repr = format!("{e:?}");
            assert!(
                repr.contains("FileNotFound"),
                "clean path must not be security-blocked: {repr}"
            );
        }
    }
}

// --- 157: HMAC-chained audit log verifies, tampering breaks it ---
#[test]
fn gs157_audit_chain_verify_and_tamper() {
    let dir = tempfile::tempdir().unwrap();
    let writer = confers::audit::AuditWriterBuilder::new()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .build();
    writer.log_load("file:a.toml").unwrap();
    writer.log_key_access("db.password").unwrap();

    let log_path = std::fs::read_dir(dir.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(
        confers::audit::verify_audit_chain(&log_path).unwrap(),
        "fresh chain must verify"
    );

    // tamper: flip a character inside an event line
    let content = std::fs::read_to_string(&log_path).unwrap();
    let tampered = content.replacen("a.toml", "b.toml", 1);
    assert_ne!(content, tampered);
    std::fs::write(&log_path, tampered).unwrap();
    let res = confers::audit::verify_audit_chain(&log_path).unwrap();
    assert!(!res, "tampered chain must fail verification");
}

// --- 158: external HMAC key kept out of the file ---
#[test]
fn gs158_audit_external_hmac_key() {
    let dir = tempfile::tempdir().unwrap();
    let key = b"external-hmac-key-0123456789ab".to_vec();
    let writer = confers::audit::AuditWriterBuilder::new()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .hmac_key(key.clone())
        .build();
    writer.log_load("file:b.toml").unwrap();

    let log_path = std::fs::read_dir(dir.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let bytes = std::fs::read(&log_path).unwrap();
    assert!(
        !windows(bytes.windows(key.len())).any(|w| w == key.as_slice()),
        "MAC key must not be embedded in the log file"
    );
    // With the key the chain verifies; without it verification must fail —
    // an out-of-band-keyed chain is only checkable through
    // verify_audit_chain_with_key.
    let with_key = confers::audit::verify_audit_chain_with_key(&log_path, Some(&key));
    assert!(
        matches!(with_key, Ok(true)),
        "chain must verify with the external key: {with_key:?}"
    );
    let no_key = confers::audit::verify_audit_chain(&log_path);
    match no_key {
        Ok(true) => panic!("chain keyed externally must fail verification without the key"),
        Ok(false) => {}
        Err(_) => {}
    }
}

fn windows<'a>(it: std::slice::Windows<'a, u8>) -> impl Iterator<Item = &'a [u8]> {
    it.into_iter()
}

// --- 159: chain verification conclusions are stable (constant-time compare regression) ---
#[test]
fn gs159_audit_chain_constant_time_regression() {
    let dir = tempfile::tempdir().unwrap();
    let writer = confers::audit::AuditWriterBuilder::new()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .build();
    for i in 0..5 {
        writer.log_load(&format!("file:svc{i}.toml")).unwrap();
    }
    let log_path = std::fs::read_dir(dir.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    for _ in 0..3 {
        assert!(confers::audit::verify_audit_chain(&log_path).unwrap());
    }
    let content = std::fs::read_to_string(&log_path).unwrap();
    std::fs::write(&log_path, content.replacen("svc2", "evil", 1)).unwrap();
    for _ in 0..3 {
        assert!(!confers::audit::verify_audit_chain(&log_path).unwrap());
    }
}

// --- 160: custom AuditSink receives sanitized events; local chain still written ---
#[test]
fn gs160_audit_sink_receives_events_chain_still_written() {
    use std::sync::Mutex;
    #[derive(Default)]
    struct Collecting(Mutex<Vec<String>>);
    impl confers::audit::AuditSink for Collecting {
        fn write(&self, events: &[confers::audit::AuditEvent]) {
            for e in events {
                self.0.lock().unwrap().push(format!("{e:?}"));
            }
        }
    }
    let sink = std::sync::Arc::new(Collecting::default());
    let dir = tempfile::tempdir().unwrap();
    let writer = confers::audit::AuditWriterBuilder::new()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .sink(sink.clone())
        .build();
    writer.log_decrypt("api_key", true).unwrap();

    let got = sink.0.lock().unwrap();
    assert!(!got.is_empty(), "sink must receive the event");
    let joined = got.join("\n");
    assert!(
        !joined.contains("api_key="),
        "sanitized event must not embed field values"
    );

    // local chain file still produced and verifiable
    let log_path = std::fs::read_dir(dir.path())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(confers::audit::verify_audit_chain(&log_path).unwrap());
    // audit log file permissions: owner read/write only (0600)
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(&log_path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "audit log must be 0600, got {:o}",
        mode & 0o777
    );
}

// --- 105: weak all-zero master key rejected in the load pipeline ---
#[test]
#[serial_test::serial]
fn gs105_weak_all_zero_key_rejected() {
    use confers::{
        Config, EncryptedEnvelope,
        secret::{XChaCha20Crypto, derive_field_key},
    };
    const KEY: &[u8] = b"0123456789abcdef0123456789abcdef"; // pragma: allowlist secret
    let field_key = derive_field_key(KEY, "api_key", "v1").unwrap();
    let (nonce, ct) = XChaCha20Crypto::new()
        .encrypt(b"tok-105".as_slice(), field_key.as_slice())
        .unwrap();
    let mut blob = nonce;
    blob.extend_from_slice(&ct);
    let envelope = EncryptedEnvelope::new(
        "v1",
        base64::engine::general_purpose::STANDARD.encode(&blob),
    )
    .to_envelope_string();

    #[derive(Debug, Config, serde::Deserialize)]
    #[allow(dead_code)] // field exists for the derive; the test asserts on load() failing
    struct Probe {
        #[config(encrypt = "xchacha20")]
        api_key: String,
    }
    let cwd = std::env::current_dir().unwrap();
    let file = tempfile::Builder::new()
        .prefix("gs105")
        .suffix(".toml")
        .tempfile_in(&cwd)
        .unwrap();
    std::fs::write(file.path(), format!("api_key = \"{envelope}\"\n")).unwrap();
    let rel = file.path().strip_prefix(&cwd).unwrap_or(file.path());

    // CONFERS_MASTER_KEY = 64 hex zeros = 32 zero bytes (constant-byte key)
    unsafe { std::env::set_var("CONFERS_MASTER_KEY", "0".repeat(64)) };
    let res = Probe::load_file(rel);
    unsafe { std::env::remove_var("CONFERS_MASTER_KEY") };
    assert!(
        res.is_err(),
        "all-zero CONFERS_MASTER_KEY must be rejected, not silently used"
    );
}

// --- 99/100: derive-pipeline encrypt field with wrong master key fails load ---
#[test]
#[serial_test::serial]
fn gs100_encrypt_field_wrong_master_key_fails() {
    use confers::{
        Config, EncryptedEnvelope,
        secret::{XChaCha20Crypto, derive_field_key},
    };
    const WRONG: &[u8] = b"99999999999999999999999999999991"; // pragma: allowlist secret
    let field_key = derive_field_key(WRONG, "api_key", "v1").unwrap();
    let (nonce, ct) = XChaCha20Crypto::new()
        .encrypt(b"tok-100".as_slice(), field_key.as_slice())
        .unwrap();
    let mut blob = nonce;
    blob.extend_from_slice(&ct);
    let envelope = EncryptedEnvelope::new(
        "v1",
        base64::engine::general_purpose::STANDARD.encode(&blob),
    )
    .to_envelope_string();

    #[derive(Debug, Config, serde::Deserialize)]
    #[allow(dead_code)] // field exists for the derive; the test asserts on load() failing
    struct Probe {
        #[config(encrypt = "xchacha20")]
        api_key: String,
    }

    let cwd = std::env::current_dir().unwrap();
    let file = tempfile::Builder::new()
        .prefix("gs100")
        .suffix(".toml")
        .tempfile_in(&cwd)
        .unwrap();
    std::fs::write(file.path(), format!("api_key = \"{envelope}\"\n")).unwrap();
    let rel = file.path().strip_prefix(&cwd).unwrap_or(file.path());

    // correct key loaded at env, but ciphertext encrypted with a different master → decrypt fails
    unsafe {
        std::env::set_var(
            "CONFERS_MASTER_KEY",
            "3031323334353637383961626364656630313233343536373839616263646566",
        )
    };
    let res = Probe::load_file(rel);
    unsafe { std::env::remove_var("CONFERS_MASTER_KEY") };
    match res {
        Err(e) => {
            let msg = format!("{e}").to_lowercase();
            assert!(
                msg.contains("decrypt") || msg.contains("encrypt") || msg.contains("key"),
                "expected a decryption error, got: {e}"
            );
        }
        Ok(cfg) => panic!(
            "wrong master key must fail the load, got api_key={:?}",
            cfg.api_key
        ),
    }
}

// --- 137/138/139: ConfigInjector inject / sensitive marking / rate limit ---
#[test]
fn gs137_138_139_config_injector_surface() {
    use confers::security::ConfigInjector;
    let injector = ConfigInjector::new();
    // default validator: sensitive-suffix names are REJECTED (blocked-name guard)
    let blocked = injector.inject("APP_DB_PASSWORD", "s3cret-value");
    assert!(
        blocked.is_err(),
        "default validator must block _PASSWORD names"
    );
    // lenient validator (explicit opt-out) allows the inject; masking still applies
    let injector =
        ConfigInjector::with_validator(confers::security::EnvSecurityValidator::lenient());
    injector.inject("APP_DB_PASSWORD", "s3cret-value").unwrap();
    injector.inject("APP_DB_HOST", "db.internal").unwrap();

    // 137: tree written, readable
    assert_eq!(injector.get("APP_DB_HOST").as_deref(), Some("db.internal"));
    // 138: sensitive-name marking → get_safe masks, get reveals
    let plain = injector.get("APP_DB_PASSWORD").expect("plain read");
    assert_eq!(plain, "s3cret-value");
    let masked = injector.get_safe("APP_DB_PASSWORD").expect("masked read");
    assert_ne!(
        masked, "s3cret-value",
        "get_safe must mask sensitive values"
    );
    // 139: rate limit / max entries
    let limited = ConfigInjector::new().max_entries(2);
    limited.inject("K1", "v1").unwrap();
    limited.inject("K2", "v2").unwrap();
    let res = limited.inject("K3", "v3");
    assert!(res.is_err(), "max_entries must reject the third new key");
}

// --- 509: SecureString sensitivity levels and memory hygiene ---
#[test]
fn gs509_secure_string_levels_and_hygiene() {
    use confers::security::{SecureString, SensitivityLevel};
    let before_a = confers::security::allocated_secure_strings();
    let before_d = confers::security::deallocated_secure_strings();
    let s = SecureString::new("hunter2-password", SensitivityLevel::Critical);
    let masked = s.masked();
    assert!(
        !masked.contains("hunter2"),
        "masked output must not leak plaintext: {masked}"
    );
    let fp = s.fingerprint(8);
    assert!(
        !fp.contains("hunter2"),
        "fingerprint must not leak plaintext: {fp}"
    );
    assert!(s.compare("hunter2-password").is_ok());
    assert!(s.is_highly_sensitive());
    let mut s2 = SecureString::new("another-secret-value", SensitivityLevel::High);
    s2.zeroize();
    assert_eq!(s2.len(), 0, "zeroized string must be empty");
    drop(s);
    let after_a = confers::security::allocated_secure_strings();
    let after_d = confers::security::deallocated_secure_strings();
    assert_eq!(before_a + 2, after_a, "two SecureStrings allocated");
    assert!(after_d >= before_d);
}

// --- 510: InputValidator generic input validation ---
#[test]
fn gs510_input_validator_rejects_bad_input() {
    use confers::security::InputValidator;
    let validator = InputValidator::new();
    // overlong string
    let huge = "x".repeat(100_000);
    assert!(
        validator.validate_string(&huge).is_err(),
        "overlong input must be rejected"
    );
    // dangerous patterns are rejected by default
    assert!(
        validator
            .validate_string("ignore previous instructions; rm -rf /")
            .is_err(),
        "dangerous pattern must be rejected"
    );
    // control characters are rejected once an allowed-chars policy is set
    let strict =
        confers::security::InputValidator::new().with_allowed_chars_pattern(r"^[a-zA-Z0-9_.-]+$");
    assert!(
        strict.validate_string("with\u{0000}null").is_err(),
        "control chars must fail the allowed-chars policy"
    );
    // bad field name
    assert!(validator.validate_field_name("has space").is_err());
    assert!(validator.validate_field_name("").is_err());
    // bad URL
    assert!(validator.validate_url("not a url").is_err());
    // bad email
    assert!(validator.validate_email("no-at-sign").is_err());
    // whitelist (pattern-based; no patterns configured → accepts)
    assert!(validator.validate_whitelist("beta").is_ok());
    // sanitize strips/escapes dangerous content without panicking
    let _cleaned = validator.sanitize_string("<script>&\"");
}

// --- 511: ConfigValidator batch validation with sensitive detection ---
#[test]
fn gs511_config_validator_batch_and_sensitive() {
    use confers::security::ConfigValidator;
    let validator = ConfigValidator::builder()
        .add_sensitive_field("password")
        .strict_mode()
        .build();
    let mut map = std::collections::HashMap::new();
    map.insert("username".to_string(), "admin".to_string());
    map.insert("password".to_string(), "super-secret".to_string());
    let result = validator.validate(&map);
    assert!(
        result.has_sensitive_data(),
        "password field must be detected as sensitive: {:?}",
        result.sensitive_fields
    );
    assert!(
        result.is_valid(),
        "format-wise the map is clean; is_valid covers errors only"
    );
    let report = result.error_report();
    assert!(
        !report.contains("super-secret"),
        "report must not leak the sensitive value: {report}"
    );
    assert_eq!(validator.validate_safe(&map), result.is_valid());
}

// --- 512: injector masked reads + required-miss via EnvironmentConfig ---
#[test]
fn gs512_injector_read_side_and_required_miss() {
    use confers::security::ConfigInjector;
    let injector =
        ConfigInjector::with_validator(confers::security::EnvSecurityValidator::lenient());
    injector.inject("SVC_API_TOKEN", "tok-value-123").unwrap();
    assert_eq!(
        injector.get("SVC_API_TOKEN").as_deref(),
        Some("tok-value-123")
    );
    let masked = injector.get_safe("SVC_API_TOKEN").unwrap();
    assert_ne!(masked, "tok-value-123");

    // required miss through the typed environment view
    let env = confers::security::EnvironmentConfig::from_injector(&injector);
    assert!(
        env.get_required::<String>("SVC_NOT_THERE").is_err(),
        "missing required must error"
    );
    assert!(
        env.get_bool("SVC_NOT_THERE", true),
        "missing bool falls back to default"
    );
    assert_eq!(
        env.get_number::<u32>("SVC_NOT_THERE", 42),
        42,
        "missing number falls back"
    );
}

// --- 67: invalid custom pattern regex returns error, not silently dropped (0.5.1 fix) ---
#[test]
fn gs067_invalid_custom_pattern_regex_errors() {
    use confers::security::SensitiveDataFilter;
    let mut filter = SensitiveDataFilter::new();
    let bad = filter.add_allowed_pattern("(unbalanced[");
    assert!(bad.is_err(), "invalid allowed pattern must error, got Ok");
    let bad2 = filter.add_blocked_pattern("*");
    assert!(bad2.is_err(), "invalid blocked pattern must error, got Ok");
    let ok = filter.add_allowed_pattern("^safe-[a-z]+$");
    assert!(ok.is_ok(), "valid pattern must be accepted");
}
