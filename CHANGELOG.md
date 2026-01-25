# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-01-25

### Security
- Internal function visibility hardening (pub(crate) for internal helpers)
- TlsConfig refactored to use builder pattern
- Clippy code style fixes applied

### Tests
- All 167 unit tests passing
- All 34 doc tests passing
- cargo clippy --all-features passing
- cargo deny check passing

### Changed
- FileFormat unified across the codebase

## [0.2.1] - 2026-01-17

### Security
- **Enhanced Audit Logging System**: Added comprehensive audit logging with event classification, integrity protection, log rotation, and query capabilities
- **Enhanced Configuration Validation**: Implemented advanced validation system with range, dependency, format, and consistency validators
- **Security Documentation**: Added comprehensive security documentation in API reference and user guide
- **Security Annotations**: Added security annotations to sensitive API methods (encryption, key management, audit logging, configuration validation)
- **Security Examples**: Added security examples for audit logging, configuration validation, and key management

### Added
- Audit event types and priorities (ConfigLoad, KeyRotation, SecurityViolation, etc.)
- Audit event generator with metadata tracking
- Audit log writer with HMAC integrity protection
- Log rotation and archival with gzip compression
- Audit log query interface with filtering and pagination
- AdvancedConfigValidator trait for extensible validation
- ValidationEngine with priority-based validator execution
- RangeFieldValidator for numeric range validation
- DependencyValidator for field dependency validation
- FormatValidator for string format validation (email, URL, etc.)
- ConsistencyValidator for cross-field consistency validation
- CachedValidationEngine with LRU cache for performance
- Comprehensive security documentation in API_REFERENCE.md
- Security configuration best practices in USER_GUIDE.md
- Security examples in examples/src/06-encryption/ and examples/src/08-audit/
- Advanced validation example in examples/src/02-validation/02-validation-advanced_validation.rs

### Changed
- Added EncryptionError and DecryptionError to ConfigError enum
- Added std::io::Write import to src/audit/mod.rs
- Fixed Arc::clone usage for SecureString in src/core/loader.rs
- Updated Cargo.toml with flate2 dependency for log compression
- Updated API_REFERENCE.md with comprehensive security notes
- Updated USER_GUIDE.md with security configuration best practices
- Updated examples with complete security demonstrations

### Fixed
- Fixed compilation errors with Arc<SecureString> type annotations
- Fixed unused import warnings in src/audit/mod.rs and src/validator/mod.rs
- Fixed documentation tests with proper imports and types

### Tests
- All 204 tests passing (178 unit tests + 26 doc tests)
- All security tests passing (93 tests)
- All audit tests passing (6 tests)
- All validator tests passing (13 tests)

## [0.2.0] - 2026-01-16

### Security
- **Internal Implementation Protection**: Privatized sensitive fields in `RemoteConfig` and `ConfigLoader` to prevent accidental exposure.
- **Sensitive Data Isolation**: Replaced `String` with `Arc<SecureString>` for sensitive fields (`password`, `token`, `bearer_token`) in `HttpProvider` and `RemoteConfig`.
- **Access Control**: Introduced secure Builder patterns for `EnvironmentValidationConfig` and `RemoteConfig`, enforcing secure construction via `with_auth_secure` and `with_bearer_token_secure`.
- **SSRF Protection**: Enhanced `HttpProvider` to validate URLs in all loading methods (`load`, `load_sync`), preventing potential SSRF attacks even if internal state is mutated.
- **Leakage Prevention**: Fixed potential sensitive data leakage in HTTP provider by correctly handling `SecureString` during request authentication (avoiding masked output).
- Add comprehensive security module with production-ready features:
  - SecureString with automatic memory zeroization for sensitive data
  - ConfigInjector for secure runtime configuration injection
  - InputValidator for SQL/command injection prevention
  - ErrorSanitizer for sensitive data redaction in error messages
- Add sensitive data detection and warnings in proc-macros:
  - Detect hardcoded passwords, tokens, and private keys at compile time
  - Emit runtime warnings to guide users toward safer alternatives
  - Implement input length limits to prevent DoS attacks
- Fix SSRF test mode bypass - only allow localhost bypass in non-production environments
- Fix environment variable injection - add validation before substitution
- Fix path traversal protection with comprehensive checks:
  - Detect traversal patterns including URL encoding and Windows paths
  - Block access to sensitive system directories (/etc, /usr, /var/log, etc.)
  - Prevent symlink attacks via canonicalization
- Enhance security config injector with improved validation
- Refactor error sanitization for better security
- Improve input validation logic

### Fixed
- Fixed compilation errors in `Config` derive macro by automatically implementing `OptionalValidate` trait when validation feature is disabled.
- Resolved duplicate method definitions in `ConfigLoader`.

### Added
- Create unified file format detection module (eliminates 4 duplicate implementations)
- Add comprehensive security tests (800+ lines of test coverage)

### Changed
- Enhance error handling with comprehensive error types
- Optimize HTTP provider for remote configuration
- Rename `resource/` directory to `docs/image/` for better organization
- Update documentation with cleaner styling and correct links
- Reduce code duplication by ~120 lines
- Centralize format detection logic in utils/file_format.rs
- **BREAKING**: Default features changed from `["derive", "validation", "cli"]` to `["derive"]` for minimal dependency footprint
- Made `rustls` optional (now only enabled with `remote` feature)
- Made `chrono`, `sysinfo`, `lru` optional dependencies (moved to `encryption` and `monitoring` features)
- Removed unused `num_cpus` dependency
- Added feature presets for easier configuration:
  - `minimal` - Only configuration loading
  - `recommended` - Configuration loading + validation
  - `dev` - Development configuration with all tools
  - `production` - Production-ready configuration
  - `full` - All features enabled
- Added conditional compilation for all optional features to minimize compilation time and binary size
- Updated `remote` feature to include `rustls`, `rustls-pki-types`, `tokio-rustls`, `failsafe`, and `base64`
- Updated `encryption` feature to include `lru` and `chrono`
- Fixed `RefreshKind::new()` to `RefreshKind::nothing()` for sysinfo compatibility

### Dependencies Updated
- Updated all dependencies to their latest stable versions
- Upgraded `lru` from 0.12 to 0.16.3 to fix soundness issue (RUSTSEC-2026-0002)
- Updated core dependencies: tokio 1.48 → 1.49, serde, validator, schemars, thiserror, clap, etc.
- All 108 tests pass with updated dependencies

## [0.1.1] - 2026-01-02

### Security
- Add DNS rebinding protection to SSRF validation to prevent SSRF attacks via hostname resolution
- Add safe_display() method to ConfigError to sanitize sensitive information from error messages
- Mask key IDs in error messages to prevent sensitive data leakage

### Fixed
- Increase default memory limit from 10MB to 512MB to prevent production outages
- Make HTTP request timeouts configurable (default 30s) for better performance control
- Replace RwLock unwrap() calls with proper error handling to prevent panics
- Update validator registry methods to return Results instead of panicking

### Added
- Add nonce cache monitoring methods (usage_percent, cache_stats) for production observability
- Add warning for low memory limits (< 100MB) in ConfigLoader

### Changed
- Simplified boolean expressions in SSRF validation (Clippy improvements)
- Improved code formatting and documentation

## [0.1.0] - 2025-12-27

### Added
- Type-safe configuration management with derive macro
- Multi-format support (TOML, YAML, JSON, INI)
- Environment variable override support
- Built-in validation system integration
- JSON Schema generation
- File monitoring and hot-reload support
- Encrypted storage for sensitive configurations
- Audit logging for configuration access and changes
- Remote configuration support (etcd, Consul, HTTP)
- CLI tool with multiple commands (encrypt, validate, diff, generate, etc.)

### Changed
- Initial release
- Improved documentation and examples

### Security
- Secure memory cleanup
- AES encryption for sensitive data
- PBKDF2 key derivation

### Thanks
- Thanks to all contributors and the Rust community
