# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-01-02

### Added
- Nonce reuse detection with LRU cache in encryption module for enhanced security
- SecureKey type with automatic memory zeroing for sensitive data protection
- SSRF protection utilities to prevent server-side request forgery attacks
- ConfigMap trait refactored to use serde_json::Value for better flexibility
- Simplified macro code generation for improved compile-time performance

### Changed
- Performance test threshold adjustments for better CI/CD reliability
- License clarifications and advisories for compliance improvements
- Enhanced CLI tool documentation with complete command reference

### Security
- Improved nonce management with LRU cache-based reuse detection
- Automatic memory cleanup for SecureKey type to prevent data leakage
- SSRF protection utilities to validate and sanitize remote URLs

### Documentation
- Updated USER_GUIDE.md with comprehensive CLI tool documentation
- Added detailed command references for diff, generate, validate, encrypt, wizard, and key commands

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
