# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
