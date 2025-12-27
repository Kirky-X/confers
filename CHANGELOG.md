# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
