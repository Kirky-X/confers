# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-01-16

### Security
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

### Added
- Create unified file format detection module (eliminates 4 duplicate implementations)
- Add comprehensive security tests (800+ lines of test coverage)

### Changed
- Enhance error handling with comprehensive error types
- Optimize HTTP provider for remote configuration
- Rename `image/` directory to `resource/` for better naming convention
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
