# 🧪 Confers Test Scenario Matrix

> Applicable version: confers **0.6.0** (workspace, Rust 1.97.1 / edition 2024)
> Purpose: the first step of the unified E2E acceptance effort across the 7 repositories: enumerate all acceptance scenarios first, then solidify them one by one into E2E tests under `tests/e2e/` according to this document.
> Authoritative inputs (read-only verification): `Cargo.toml [features]`, the export surface of `src/lib.rs`, subcommands in `src/cli/mod.rs`, the attribute table in `macros/src/parse.rs`, `tests/{core,security,remote,watcher,cli}/`, the 98 `#[cfg(test)]` modules inside src, the 22 examples in `examples/`, and `docker-compose.test.yml`.
> All referenced existing test names were verified with `grep` to exist.

## 📋 Table of Contents


- [Reading Conventions](#reading-conventions)
- [Reading Conventions](#reading-conventions)
-   [Key Findings (from the inventory at writing time)](#key-findings-from-the-inventory-at-writing-time)
- [1. Overview: Feature Domains × feature Groups](#1-overview-feature-domains--feature-groups)
- [2. Scenario Matrix](#2-scenario-matrix)
- [2. Scenario Matrix](#2-scenario-matrix)
-   [2.1 Format Parsing and Loader (FMT, 22 scenarios)](#21-format-parsing-and-loader-fmt-22-scenarios)
-   [2.2 Config Building / Source Chain / Merging (BLD, 28 scenarios)](#22-config-building--source-chain--merging-bld-28-scenarios)
-   [2.3 Validation (VAL, 8 scenarios)](#23-validation-val-8-scenarios)
-   [2.4 Interpolation (IPL, 9 scenarios)](#24-interpolation-ipl-9-scenarios)
-   [2.5 Encryption and Secret (ENC, 23 scenarios)](#25-encryption-and-secret-enc-23-scenarios)
-   [2.6 Key Management (KEY, 13 scenarios)](#26-key-management-key-13-scenarios)
-   [2.7 Security Rules and Injection (SEC, 18 scenarios)](#27-security-rules-and-injection-sec-18-scenarios)
-   [2.8 Audit (AUD, 11 scenarios)](#28-audit-aud-11-scenarios)
-   [2.9 File Hot Reload (WAT, 18 scenarios)](#29-file-hot-reload-wat-18-scenarios)
-   [2.10 Progressive Reload (PGR, 12 scenarios)](#210-progressive-reload-pgr-12-scenarios)
-   [2.11 Dynamic Fields (DYN, 14 scenarios)](#211-dynamic-fields-dyn-14-scenarios)
-   [2.12 Feature Toggle (TGL, 6 scenarios)](#212-feature-toggle-tgl-6-scenarios)
-   [2.13 Version Migration (MIG, 12 scenarios)](#213-version-migration-mig-12-scenarios)
-   [2.14 Snapshot (SNP, 10 scenarios)](#214-snapshot-snp-10-scenarios)
-   [2.15 Modules / Config Groups (MOD, 11 scenarios)](#215-modules--config-groups-mod-11-scenarios)
-   [2.16 Context Awareness (CTX, 9 scenarios)](#216-context-awareness-ctx-9-scenarios)
-   [2.17 HTTP Polled Remote Source (REM, 11 scenarios)](#217-http-polled-remote-source-rem-11-scenarios)
-   [2.18 etcd (ETC, 10 scenarios)](#218-etcd-etc-10-scenarios)
-   [2.19 Consul (CSL, 8 scenarios)](#219-consul-csl-8-scenarios)
-   [2.20 Config Bus — In-Process (BUS, 10 scenarios)](#220-config-bus--in-process-bus-10-scenarios)
-   [2.21 NATS Bus (NAT, 7 scenarios)](#221-nats-bus-nat-7-scenarios)
-   [2.22 Redis Bus (RDS, 7 scenarios)](#222-redis-bus-rds-7-scenarios)
-   [2.23 Schema (SCH, 5 scenarios)](#223-schema-sch-5-scenarios)
-   [2.24 CLI (CLI, 24 scenarios, binary `confers`, `cargo run --features cli -- …` or assert_cmd form)](#224-cli-cli-24-scenarios-binary-confers-cargo-run---features-cli-----or-assert_cmd-form)
-   [2.25 Derive Macros and Attributes (MAC, 18 scenarios)](#225-derive-macros-and-attributes-mac-18-scenarios)
-   [2.26 Feature Combination Interactions (CMP, 17 scenarios)](#226-feature-combination-interactions-cmp-17-scenarios)
-   [2.27 Concurrency and Races (CCY, 8 scenarios)](#227-concurrency-and-races-ccy-8-scenarios)
-   [2.28 Feature Preset Compile Matrix (PRS, 8 scenarios)](#228-feature-preset-compile-matrix-prs-8-scenarios)
- [3. Feature Exclusivity / Combination Matrix](#3-feature-exclusivity--combination-matrix)
- [3. Feature Exclusivity / Combination Matrix](#3-feature-exclusivity--combination-matrix)
-   [3.1 Dependency Chains (fixed by Cargo.toml `[features]`; the parts combination tests need not verify separately)](#31-dependency-chains-fixed-by-cargotoml-features-the-parts-combination-tests-need-not-verify-separately)
-   [3.2 Combination Test Matrix (input basis for CMP-01…17)](#32-combination-test-matrix-input-basis-for-cmp-0117)
-   [3.3 Preset Coverage Relations (preset → expansion list, verified by PRS-02…07)](#33-preset-coverage-relations-preset--expansion-list-verified-by-prs-0207)
- [4. Docker Service Requirements Summary](#4-docker-service-requirements-summary)
- [4. Docker Service Requirements Summary](#4-docker-service-requirements-summary)
- [5. Execution Plan](#5-execution-plan)
- [5. Execution Plan](#5-execution-plan)
-   [5.1 Layers and Commands](#51-layers-and-commands)
-   [5.1 Layers and Commands](#51-layers-and-commands)
-   [5.2 examples Run Checklist (22 total, `cargo run -p confers-examples --bin <name>`, confers dependency pinned to `features=["full"]`)](#52-examples-run-checklist-22-total-cargo-run--p-confers-examples---bin-name-confers-dependency-pinned-to-featuresfull)
-   [5.3 Scenario ID → E2E File Mapping Summary](#53-scenario-id--e2e-file-mapping-summary)
-   [5.4 Suggested Execution Order](#54-suggested-execution-order)
-   [5.4 Suggested Execution Order](#54-suggested-execution-order)
- [6. Statistics Summary](#6-statistics-summary)
- [6. Statistics Summary](#6-statistics-summary)


---

## Reading Conventions

- **Type**: `Normal` (expected behavior under legal input) / `Negative` (erroneous input or fault injection, must yield a clear error) / `Boundary` (extreme values, concurrency, races, combination edges).
- **Existing coverage**: `file::test_name` denotes an existing test (including inline test modules under `src/**`); `None → to add` denotes no existing coverage. (Historical note: at writing time `⚠dead file` was used to mark test files that were unregistered and never executed; that marker has been fully removed, see Key Findings-1 below.)
- **E2E target**: the intended file under `tests/e2e/` (to be created in the follow-up implementation phase).
- Required service values: `None` / `Local file` / `NATS(4222)` / `Redis(16379)` / `etcd(2379)` / `Consul(8500)` / `Nacos(8848)` (ports in §4).

### Key Findings (from the inventory at writing time)

1. **[Resolved] Dead-file issue is fixed**: `tests/core/error.rs` (42 tests) is now registered in `tests/core/mod.rs` (`mod error;`) and runs normally with `cargo test --test core`; `tests/core/encryption.rs`, which duplicated `tests/security/encryption.rs`, has been deleted. `tests/core/mod.rs` now registers 14 test submodules (context/coverage/derive/dynamic/env_types/error/load/merge/migration/modules/nested_deserialize/progressive/snapshot_extra/toggle, where snapshot_extra is gated by `cfg(feature = "snapshot")`) + common. References to `tests/core/error.rs` in the body of this document are to be understood as registered.
2. `tests/remote/remote.rs` contains source tests that duplicate `tests/remote/etcd.rs` and `tests/remote/consul.rs` (both compile and run).
3. `Cargo.toml` declares `autotests = false`; new E2E integration tests must **explicitly add an `[[test]]` section** and declare `required-features` as needed.
4. Empty-file `tests/core/load.rs::test_load_empty_file` etc. confirmed: an empty file parses into an empty Map rather than an error.

---

## 1. Overview: Feature Domains × feature Groups

| # | Feature Domain | Scenario ID Prefix | Features involved (Cargo.toml names) | Main Public API |
|---|--------|-------------|------------------------------|--------------|
| 1 | Format parsing and Loader | FMT | toml, json, yaml, ini, dotenv (default=toml+json+dotenv; env is a compatibility alias pending removal) | `loader::{Format, LoaderConfig, load_file, parse_content, detect_format_from_path, detect_format_from_content, parse_toml/json/yaml/ini}` |
| 2 | Config building / Source chain / Merging | BLD | default + remote(AsyncSource), snapshot(with_snapshot) | `config::{config, ConfigBuilder, ConfigLimits, SourceChainBuilder, DefaultSource, EnvSource, FileSource, MemorySource, Source, SourceKind}`, `merger::MergeStrategy`, `interface::{ConfigConnector/Reader/Writer/Provider/Ext}`, `new_in_memory()` |
| 3 | Validation | VAL | validation | `validator::{Validate, ValidationResult, ValidationRule}` |
| 4 | Interpolation | IPL | interpolation | `interpolation::{interpolate, interpolate_tracked, InterpolationConfig, InterpolationContext, InterpolationResult, InterpolationWarning}` |
| 5 | Encryption and Secret | ENC | encryption (security is decoupled: it does not imply encryption) | `secret::{SecretString, SecretBytes, XChaCha20Crypto, CryptoError, derive_field_key, EnvKeyProvider(+Builder), SecretKeyProvider, FileKeyProvider(+Builder), VaultKeyProvider(+Builder), KeyRegistry(+Builder), ZeroizingBytes}` |
| 6 | Key management | KEY | key (→encryption) | `key::{KeyManager, KeyRing, KeyBundle, KeyMetadata, KeyRotationSchedule, RotationPlan, KeyStatus}` |
| 7 | Security rules and injection | SEC | security, security-rules (security-rules→security; does not imply encryption) | `security::{EnvSecurityValidator, EnvironmentValidationConfig, config_injector, error_sanitization}`, `security::rules::{SecurityValidatorRegistry, CorsValidator, SsrfValidator, JwtSecretValidator, TlsConfigValidator, SecurityReport, SecurityViolation, ViolationSeverity}` |
| 8 | Audit | AUD | audit | `audit::{AuditEvent, AuditLevel, AuditConfig(+Builder), AuditWriter(+Builder)}` |
| 9 | File hot reload | WAT | watch | `watcher::{FsWatcher, MultiFsWatcher, WatcherConfig(+Builder), WatcherGuard, AdaptiveDebouncer}` |
| 10 | Progressive reload | PGR | progressive-reload (→watch) | `watcher::{ProgressiveReloader(+Builder), ReloadHealthCheck, HealthStatus, ReloadOutcome}` |
| 11 | Dynamic fields | DYN | dynamic (+watch's FieldWatcher) | `dynamic::{DynamicField(+Builder), CallbackGuard, FieldWatcher}` |
| 12 | Feature toggle | TGL | feature-toggle | `toggle::{FeatureToggleRegistry, FeatureInfo}` |
| 13 | Version migration | MIG | migration | `migration::{Versioned, MigrationRegistry, MigrationFn, MigrationOnReload}`, derive macro `ConfigMigration` |
| 14 | Snapshot | SNP | snapshot (→dynamic,json,toml,yaml) | `snapshot::{SnapshotManager, SnapshotConfig, SnapshotFormat, SnapshotInfo}`, `ConfigBuilder::with_snapshot` |
| 15 | Modules / config groups | MOD | modules (→toml) | `modules::{ModuleConfig, ModuleRegistry}`, derive macro `ConfigModules` |
| 16 | Context awareness | CTX | context-aware | `context::{ContextAwareField(+Builder), ContextRule, ContextValue, EvaluationContext}` |
| 17 | HTTP polled remote source | REM | remote | `remote::{HttpPolledSource(+Builder), PolledSource}`, `config::AsyncSource` |
| 18 | etcd | ETC | etcd (→remote) | etcd source builder (endpoint/auth/prefix/interval/tls) |
| 19 | Consul | CSL | consul (→remote) | consul source builder (address/token/prefix/interval/tls, max_response_bytes/max_kv_entries) |
| 20 | Config bus (in-process) | BUS | config-bus | `bus::{ConfigBus, InMemoryBus, BusBuilder, ConfigChangeEvent, BusEventLimiter}` |
| 21 | NATS bus | NAT | nats-bus (→config-bus) | `bus::NatsConfigBus, NatsBusBuilder` |
| 22 | Redis bus | RDS | redis-bus (→config-bus) | `bus::RedisConfigBus, RedisBusBuilder` |
| 23 | Schema | SCH | schema, typescript-schema | derive macro `ConfigSchema`, `schema::TypeScriptGenerator` |
| 24 | CLI | CLI | cli (→toml,json,yaml) | `cli::run::<T>`, binary `confers` (Inspect/Validate/Export/Diff/Snapshot) |
| 25 | Derive macros and attributes | MAC | — (works with each feature) | `Config`, `ConfigClap`, `ConfigMigration`, `ConfigModules`, `ConfigSchema`; struct attributes `validate/env_prefix/app_name/strict/watch/version/profile/profile_env`; field attributes `default/description/name/name_env/name_clap_long/name_clap_short/sensitive/encrypt/flatten/skip/interpolate/merge_strategy/dynamic/module_group` |
| 26 | Feature combination interactions | CMP | multiple features stacked | — |
| 27 | Concurrency and races | CCY | dynamic, watch, feature-toggle, config-bus, snapshot, audit | — |
| 28 | Feature preset compile matrix | PRS | recommended/dev/production/full/minimal/distributed + single features | `cargo check --features …` |

Total scenarios: **357**; programmatic statistics by type, required service, and other dimensions are in [6. Statistics Summary](#6-statistics-summary).
Total scenarios: **357**; programmatic statistics by type, required service, and other dimensions are in [6. Statistics Summary](#6-statistics-summary).

---

## 2. Scenario Matrix

### 2.1 Format Parsing and Loader (FMT, 22 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| FMT-01 | A valid TOML file parsed via `load_file` into a Map, keys/values matching the file | Normal | toml | Local file | `tests/core/load.rs::test_load_with_format_detection` | tests/e2e/format_e2e.rs |
| FMT-02 | Valid JSON file parsed; nested objects/arrays keep their structure | Normal | json | Local file | `tests/core/load.rs::test_load_array_values`, `tests/core/nested_deserialize.rs::test_nested_json_deserializes_into_struct` | tests/e2e/format_e2e.rs |
| FMT-03 | Valid YAML file parsed (including nested map) | Normal | yaml | Local file | `tests/core/nested_deserialize.rs::test_nested_yaml_deserializes_into_struct` | tests/e2e/format_e2e.rs |
| FMT-04 | Valid INI file parsed into flat key-values (ini has no nesting semantics; verifies [section] prefixing) | Normal | ini | Local file | Inline src tests in `src/impl_/loader.rs` | tests/e2e/format_e2e.rs |
| FMT-05 | `detect_format_from_path`: extensions .toml/.json/.yaml/.yml/.ini each return the matching Format | Normal | toml,json,yaml,ini | None | `tests/core/coverage.rs::test_detect_from_path_cases` | tests/e2e/format_e2e.rs |
| FMT-06 | `detect_format_from_content`: extension-less files sniffed by content (`{`/`key =`/`key:`) | Boundary | toml,json,yaml | None | `tests/core/coverage.rs::test_detect_from_content_formats` | tests/e2e/format_e2e.rs |
| FMT-07 | TOML syntax error (e.g. `key =`) → `ConfigError` with `ConfigErrorCode=2300(CONFIG_PARSE_ERROR)`, carrying row/column `ParseLocation` | Negative | toml | Local file | `tests/core/load.rs::test_load_invalid_toml`; `tests/core/error.rs::test_invalid_toml_format_error/test_parse_error_location` | tests/e2e/format_e2e.rs |
| FMT-08 | JSON syntax error (truncated JSON) → parse error with position | Negative | json | Local file | `tests/core/load.rs::test_load_invalid_json`; `tests/core/error.rs::test_invalid_json_format_error` | tests/e2e/format_e2e.rs |
| FMT-09 | YAML syntax error (tab indentation etc.) → parse error | Negative | yaml | Local file | `tests/core/error.rs::test_invalid_yaml_format_error` | tests/e2e/format_e2e.rs |
| FMT-10 | Empty file (0 bytes) → succeeds with an empty Map, no error | Boundary | toml | Local file | `tests/core/load.rs::test_load_empty_file` | tests/e2e/format_e2e.rs |
| FMT-11 | File exceeds `LoaderConfig::max_size` → size-limit error (2400 family), not read into memory | Negative | toml | Local file | `tests/core/error.rs::test_size_limit_exceeded_error` (unit level); no loader-level integration → to add | tests/e2e/format_e2e.rs |
| FMT-12 | `load_file` on a nonexistent path → `ConfigFileNotFound(2200)`, error carries source info | Negative | default | Local file | `tests/core/load.rs::test_load_file_not_found`; `tests/core/error.rs::test_file_not_found_with_source`; E2E: `tests/e2e/error_scenarios_e2e.rs::s12_missing_file_reports_not_found` (actual code `ErrorCode::FileNotFound(1)`; 2200 belongs to the `ConfigErrorCode` family; constants of both families are pinned in tests) | tests/e2e/format_e2e.rs |
| FMT-13 | Path traversal: `config/../../etc/passwd.toml` rejected by `normalize_and_validate_path` | Negative | default | Local file | Inline src in `src/impl_/loader.rs` (`check_path_traversal_attempt` tests) | tests/e2e/format_e2e.rs |
| FMT-14 | Symlink pointing outside allowed_dir → rejected by default; `no_symlink_check()` allows it | Negative | default | Local file | Inline src tests in `src/impl_/loader.rs` | tests/e2e/format_e2e.rs |
| FMT-15 | Absolute paths rejected by default; `allow_absolute()` / `ConfigBuilder::allow_absolute_paths()` allow them | Negative | default | Local file | `tests/cli/commands.rs::test_diff_command` (absolute-path bail branch); inline src loader tests | tests/e2e/format_e2e.rs |
| FMT-16 | Unknown extension and un-sniffable content → `Format::try_parse`/detect returns None, caller reports "unknown format" | Negative | default | Local file | `tests/core/coverage.rs::test_format_try_parse_all` (reverse assertion) | tests/e2e/format_e2e.rs |
| FMT-17 | `.env` file loaded as an env source via dotenvy; `dotenv` is the canonical name, `env` the compatibility alias | Normal | env,dotenv | Local file | `tests/cli/commands.rs::test_env_file_loading`, the family of `tests/core/derive.rs::test_env_file_loading` | tests/e2e/format_e2e.rs |
| FMT-18 | INI supports one section level only: deeply nested input neither loses data nor panics (degrades to string) | Boundary | ini | Local file | Inline src loader tests (parse_ini branch) → partial, to add assertions | tests/e2e/format_e2e.rs |
| FMT-19 | TOML/JSON files with a UTF-8 BOM parse (or error explicitly per the implementation; pin one of the two) | Boundary | toml,json | Local file | None → to add | tests/e2e/format_e2e.rs |
| FMT-20 | For identical content, `load_file(path)` and `parse_content(content, detect(path))` produce equivalent results (values + source) | Normal | toml,json | Local file | `tests/core/load.rs::test_load_with_format_detection` (partial) → to add equivalence assertions | tests/e2e/format_e2e.rs |
| FMT-21 | Same config semantics in three formats: TOML/JSON/YAML each written with the same nested structure, parsed Maps deeply equal | Normal | toml,json,yaml | Local file | `tests/core/nested_deserialize.rs` (3 cases) | tests/e2e/format_e2e.rs |
| FMT-22 | With the toml/json feature disabled, `parse_toml/parse_json` return a placeholder "format not compiled" error (cfg branch) | Negative | (feature disabled) | None | Inline src stub branch in `src/impl_/loader.rs` | tests/e2e/format_e2e.rs (no-default-features compile) |

### 2.2 Config Building / Source Chain / Merging (BLD, 28 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| BLD-01 | `config::<T>().file(f).build()` deserializes into a typed struct with every field hit | Normal | default | Local file | `tests/core/derive.rs::test_simple_config_load`, `tests/core/load.rs::test_config_builder_creation` | tests/e2e/builder_e2e.rs |
| BLD-02 | `build_annotated()` returns `AnnotatedValue`: every leaf carries source and location | Normal | default | Local file | `tests/cli/commands.rs::test_export_with_provenance` (CLI surface); library surface → to add direct assertions | tests/e2e/builder_e2e.rs |
| BLD-03 | Single `FileSource` (`.file()`) loads successfully, SourceKind=file | Normal | default | Local file | `tests/core/coverage.rs::test_file_source_basic` | tests/e2e/builder_e2e.rs |
| BLD-04 | Two `.file()` calls: the later-added source overrides same-name keys of the earlier one | Normal | default | Local file | `tests/core/merge.rs::test_precedence_file_shadows_default` family | tests/e2e/builder_e2e.rs |
| BLD-05 | `.env()` collects `APP_xxx`-style environment variables and builds a tree per the `__`/nesting rules | Normal | env | None | `tests/core/env_types.rs::test_env_vars_deserialize_into_typed_struct`, `test_env_deep_nesting_builds_nested_tree` | tests/e2e/builder_e2e.rs |
| BLD-06 | `.env_prefix("MYAPP_")` collects only prefixed variables and strips the prefix | Normal | env | None | `tests/core/derive.rs::test_prefixed_config_env_mapping` | tests/e2e/builder_e2e.rs |
| BLD-07 | `.defaults(map)` supplies the full set of defaults | Normal | default | None | `tests/core/merge.rs::test_precedence_default_file_env_memory` | tests/e2e/builder_e2e.rs |
| BLD-08 | `.default(key, value)` single-key default, overridable by file/env | Normal | default | None | `tests/core/derive.rs::test_simple_config_default` | tests/e2e/builder_e2e.rs |
| BLD-09 | `.memory(map)` injects a memory source | Normal | default | None | `tests/core/coverage.rs::test_memory_source_basic/with_initial_values` | tests/e2e/builder_e2e.rs |
| BLD-10 | `.memory_priority(n)` adjusts the memory source priority | Boundary | default | None | family near `tests/core/coverage.rs::test_memory_source_not_optional`; dedicated assertion → to add | tests/e2e/builder_e2e.rs |
| BLD-11 | Full priority chain default < file < env < memory: same key defined in all four places, final value taken from memory | Normal | env | None | `tests/core/merge.rs::test_precedence_default_file_env_memory` | tests/e2e/builder_e2e.rs |
| BLD-12 | Global `MergeStrategy` five kinds: Replace/Join/Append/Prepend/JoinAppend each verified for merge result (unconditional deep merge between Maps; the DeepMerge variant no longer exists) | Normal | default | None | `tests/core/merge.rs::test_merge_replace_strategy` and 4 more | tests/e2e/builder_e2e.rs |
| BLD-13 | `.field_strategy("tags", Append)` field-level strategy overrides the global strategy | Boundary | default | None | `tests/core/merge.rs::test_field_specific_strategy` | tests/e2e/builder_e2e.rs |
| BLD-14 | `ConfigLimits::max_file_size_bytes` exceeded → `ConfigSizeLimitExceeded(2400)` | Negative | default | Local file | `tests/core/error.rs::test_size_limit_exceeded_error`; E2E: `tests/e2e/error_scenarios_e2e.rs::s36_limits_max_file_size_bytes_rejected_at_build` (build enforces it before reading sources; actual code `ErrorCode::SizeLimitExceeded(500)`, 2400 belongs to the `ConfigErrorCode` family) | tests/e2e/builder_e2e.rs |
| BLD-15 | Nesting depth beyond `max_nesting_depth` → explicit error | Negative | default | Local file | Inline src limits tests (`src/impl_/config/limits.rs`) → to add integration | tests/e2e/builder_e2e.rs |
| BLD-16 | Total key count beyond `max_total_fields` → limit-exceeded error | Negative | default | Local file | Inline src limits tests → to add integration | tests/e2e/builder_e2e.rs |
| BLD-17 | Array length beyond `max_array_length` → limit-exceeded error | Negative | default | Local file | Inline src limits tests → to add integration | tests/e2e/builder_e2e.rs |
| BLD-18 | String beyond `max_string_length` → limit-exceeded error | Negative | default | Local file | Inline src limits tests → to add integration | tests/e2e/builder_e2e.rs |
| BLD-19 | Required field missing (non-Option field without default) → `MissingField(2001)` | Negative | default | Local file | `tests/core/error.rs::test_missing_required_field_error/test_missing_nested_required_field` | tests/e2e/builder_e2e.rs |
| BLD-20 | Type mismatch (port="abc" against u16) → type error rather than panic; int↔string conversion edges each error out | Negative | default | Local file | `tests/core/error.rs::test_type_mismatch_error/test_integer_to_string_mismatch/test_string_to_integer_mismatch/test_object_to_primitive_mismatch` | tests/e2e/builder_e2e.rs |
| BLD-21 | `build_resilient()` + `fail_fast(false)`: one corrupted source does not abort; `BuildResult::Degraded` carries `SourceWarning` | Boundary | default | Local file | `tests/core/error.rs::test_build_result_degraded/test_build_result_with_warnings/test_multi_source_error_partial_config` | tests/e2e/builder_e2e.rs |
| BLD-22 | `build_with_fallback(fallback)`: when all sources fail, fall back to the default instance and mark a warning | Negative | default | Local file | Inline src builder tests (`src/impl_/config/builder.rs`) → to add integration | tests/e2e/builder_e2e.rs |
| BLD-23 | `.file_optional(nonexistent path)` → that source is skipped, build succeeds | Boundary | default | Local file | `tests/core/coverage.rs::test_file_source_optional_flag` | tests/e2e/builder_e2e.rs |
| BLD-24 | `.file(nonexistent path)` (not optional) → `ConfigFileNotFound(2200)` | Negative | default | Local file | `tests/core/load.rs::test_load_file_not_found` | tests/e2e/builder_e2e.rs |
| BLD-25 | `SourceChainBuilder` hand-assembles a custom chain of DefaultSource/EnvSource/FileSource/MemorySource | Normal | env | None | `tests/core/coverage.rs::test_source_kind_values`, inline src chain tests (`src/impl_/config/chain.rs`) | tests/e2e/builder_e2e.rs |
| BLD-26 | Merge encounters Null values: null-overrides/is-overridden semantics pinned (under Replace, null overrides scalars) | Boundary | default | None | `tests/core/merge.rs::test_merge_null_values` | tests/e2e/builder_e2e.rs |
| BLD-27 | Empty config: no sources at all + struct with all-Option fields → builds successfully with all defaults; all required → MissingField | Boundary | default | None | `tests/core/coverage.rs::test_default_source_collect_empty`, same family as `tests/cli/commands.rs::test_build_annotated_from_cli_empty_paths` | tests/e2e/builder_e2e.rs |
| BLD-28 | `new_in_memory()` factory: set/get_string/has/delete/clear/health_check/shutdown full chain | Normal | default | None | Inline src tests in `src/impl_/memory.rs`, `tests/core/dynamic.rs::test_real_config_provider` | tests/e2e/builder_e2e.rs |

### 2.3 Validation (VAL, 8 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| VAL-01 | `#[derive(Config)] #[config(validate_helper)]` valid config passes explicit `confers_validate()` | Normal | validation | Local file | `tests/core/coverage.rs::test_validate_trait_is_accessible`; end-to-end → to add | tests/e2e/validation_e2e.rs |
| VAL-02 | Invalid value (e.g. malformed email / out-of-range) fails explicit `confers_validate()`, `Err` message contains field path (`#[config(validate)]` is a no-op; build does not validate) | Negative | validation | Local file | `tests/core/error.rs::test_config_validation_failure` (a registered version also exists in inline src validator tests) | tests/e2e/validation_e2e.rs |
| VAL-03 | `ValidationRule::from_str("length(1..)")/"range(0..100)"` parse into rules | Normal | validation | None | `tests/core/coverage.rs::test_validation_rule_from_str_length/range/simple` | tests/e2e/validation_e2e.rs |
| VAL-04 | Invalid rule string → parse error (not silent ignore) | Negative | validation | None | `tests/core/coverage.rs::test_validation_rule_from_str_invalid` | tests/e2e/validation_e2e.rs |
| VAL-05 | Custom validator registered into the chain and invoked | Normal | validation | None | Inline src tests in `src/impl_/validator.rs` | tests/e2e/validation_e2e.rs |
| VAL-06 | garde report → `ConfigError` conversion preserves field info; `user_message` outputs readable text | Negative | validation | None | `tests/core/error.rs::test_validation_error_from_garde_report/test_validation_error_user_message` | tests/e2e/validation_e2e.rs |
| VAL-07 | Multiple fields violated simultaneously → errors aggregated into one result containing all fields | Boundary | validation | None | Inline src validator tests → to add integration | tests/e2e/validation_e2e.rs |
| VAL-08 | ValidationRule equality/Clone/Debug (rules are comparable as part of config) | Boundary | validation | None | `tests/core/coverage.rs::test_validation_rule_equality/test_validation_rule_clone_and_debug/test_validation_result_type_alias` | tests/e2e/validation_e2e.rs |

### 2.4 Interpolation (IPL, 9 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| IPL-01 | `${VAR}` resolved and replaced by the resolver (`interpolate("Server: ${HOST}")` → `Server: localhost`) | Normal | interpolation | None | Inline src in `src/impl_/interpolation.rs::` (assertion at line 649) | tests/e2e/interpolation_e2e.rs |
| IPL-02 | `${VAR:8080}` default-value syntax: default used when VAR is missing | Normal | interpolation | None | Inline src interpolation tests (covered by doc and tests) | tests/e2e/interpolation_e2e.rs |
| IPL-03 | Undefined variable without default → `InterpolationConfigError(2800)`/undefined variable error | Negative | interpolation | None | `tests/core/error.rs::test_undefined_variable_error` | tests/e2e/interpolation_e2e.rs |
| IPL-04 | Circular reference A→B→A → `ConfigCircularReference(2900)` | Negative | interpolation | None | `tests/core/error.rs::test_circular_reference_detection` | tests/e2e/interpolation_e2e.rs |
| IPL-05 | Self-reference `${A}` defined within A itself → circular reference reported | Negative | interpolation | None | `tests/core/error.rs::test_self_reference_error` | tests/e2e/interpolation_e2e.rs |
| IPL-06 | Nested cycle chain (3+ nodes) detected and path reported | Negative | interpolation | None | `tests/core/error.rs::test_nested_circular_reference` | tests/e2e/interpolation_e2e.rs |
| IPL-07 | Sensitive variable marking: `interpolate_tracked("${API_KEY}", …, true)` → `InterpolationResult::has_sensitive_refs/referenced("API_KEY")`; after `InterpolationContext.record`, `is_sensitive_ref` takes effect | Normal | interpolation | None | Inline src interpolation tests (sensitive assertion group) | tests/e2e/interpolation_e2e.rs |
| IPL-08 | Truncated template `"${VAR"` (unclosed) → explicit parse error, no panic | Negative | interpolation | None | Inline src interpolation tests (assertion Err around line ~798) | tests/e2e/interpolation_e2e.rs |
| IPL-09 | `examples/interpolation` full chain: file+env mixed template interpolation output (example runs through) | Boundary | interpolation,env | Local file | examples/interpolation (verified by running) | tests/e2e/combo_e2e.rs (references the example) |

### 2.5 Encryption and Secret (ENC, 23 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| ENC-01 | `XChaCha20Crypto` encrypt→decrypt roundtrip preserves content | Normal | encryption | None | `tests/security/encryption.rs::test_encrypt_decrypt_roundtrip` | tests/e2e/encryption_e2e.rs |
| ENC-02 | Encrypting the same plaintext twice yields unique nonces (different ciphertexts) | Boundary | encryption | None | `tests/security/encryption.rs::test_encrypt_produces_unique_nonces` | tests/e2e/encryption_e2e.rs |
| ENC-03 | Decrypting with the wrong key → `CryptoError`, no plaintext returned | Negative | encryption | None | `tests/security/encryption.rs::test_decrypt_with_wrong_key_fails` | tests/e2e/encryption_e2e.rs |
| ENC-04 | Decrypting with a tampered nonce → failure | Negative | encryption | None | `tests/security/encryption.rs::test_decrypt_with_wrong_nonce_fails` | tests/e2e/encryption_e2e.rs |
| ENC-05 | Key length other than 32 bytes (short/long) → construction/encryption reports `InvalidKeyLength` | Negative | encryption | None | `tests/security/encryption.rs::test_invalid_key_length_fails/test_encrypt_key_too_short/test_encrypt_key_too_long` | tests/e2e/encryption_e2e.rs |
| ENC-06 | Empty-plaintext encrypt/decrypt roundtrip | Boundary | encryption | None | `tests/security/encryption.rs::test_encrypt_empty_data` | tests/e2e/encryption_e2e.rs |
| ENC-07 | Large plaintext (MB scale) encrypt/decrypt roundtrip | Boundary | encryption | None | `tests/security/encryption.rs::test_encrypt_large_data` | tests/e2e/encryption_e2e.rs |
| ENC-08 | `derive_field_key`: deterministic for same master and same field; different fields derive different keys | Normal | encryption | None | `tests/security/encryption.rs::test_derive_field_key/test_derive_field_key_deterministic/test_derive_field_key_different_fields` | tests/e2e/encryption_e2e.rs |
| ENC-09 | `SecretString` Debug output redacted (plaintext not leaked) | Normal | encryption | None | `tests/security/encryption.rs::test_secret_string_debug_redacts` | tests/e2e/encryption_e2e.rs |
| ENC-10 | `SecretBytes` zeroizes memory after drop (test hook verifies buffer is zeroed) | Boundary | encryption | None | `tests/security/encryption.rs::test_secret_bytes_drop_zeroizes` | tests/e2e/encryption_e2e.rs |
| ENC-11 | SecretString/SecretBytes clone/len/deref/default semantics | Normal | encryption | None | `tests/security/encryption.rs::test_secret_string_*`, `test_secret_bytes_clone/len` | tests/e2e/encryption_e2e.rs |
| ENC-12 | `EnvKeyProvider`: reading a 32-byte key from an environment variable succeeds | Normal | encryption | None | `tests/security/encryption.rs::test_env_key_provider/test_env_key_provider_exact_length` | tests/e2e/encryption_e2e.rs |
| ENC-13 | `EnvKeyProvider`: environment variable missing → explicit error | Negative | encryption | None | `tests/security/encryption.rs::test_env_key_provider_missing_var/test_env_key_provider_builder_missing_env_var/test_env_key_provider_builder_no_env_var_set` | tests/e2e/encryption_e2e.rs |
| ENC-14 | Key too short/long → rejected with the expected length reported | Negative | encryption | None | `tests/security/encryption.rs::test_env_key_provider_too_short_key/test_env_key_provider_too_long_key` | tests/e2e/encryption_e2e.rs |
| ENC-15 | `EnvKeyProviderBuilder` chained construction (including custom var name) | Normal | encryption | None | `tests/security/encryption.rs::test_env_key_provider_builder` | tests/e2e/encryption_e2e.rs |
| ENC-16 | `FileKeyProvider`: reads a key from a file; missing file/permission error → error | Normal/Negative | encryption | Local file | Inline src tests in `src/secret/providers.rs` | tests/e2e/encryption_e2e.rs |
| ENC-17 | `VaultKeyProvider` (encryption+remote combination): unreachable Vault address → connection error rather than panic | Negative | encryption,remote | None (unreachable address) | Inline src tests in `src/secret/providers.rs` cfg(remote) | tests/e2e/encryption_e2e.rs |
| ENC-18 | `KeyRegistry`: after rotate, `try_all_keys` decrypts old data with the old key and encrypts new data with the new key | Normal | encryption | None | `tests/security/security.rs::test_key_registry_rotation/test_key_registry_try_all_keys` | tests/e2e/encryption_e2e.rs |
| ENC-19 | Full workflow: provider→derive_field_key→encrypt→decrypt (end-to-end isomorphic to `test_full_encryption_workflow`) | Normal | encryption | None | `tests/security/encryption.rs::test_full_encryption_workflow/test_secret_string_with_encryption` | tests/e2e/encryption_e2e.rs |
| ENC-20 | Derive macro `#[config(encrypt = "xchacha20")]` field: ciphertext decrypted and injected at config load; wrong key → load failure | Normal/Negative | encryption,macros | Local file | Related inline src in `macros/src/codegen/security.rs`; integration → to add | tests/e2e/encryption_e2e.rs |
| ENC-21 | `encrypt = "aes256-gcm"`: syntax accepted but not implemented at runtime → macro-expansion compile error hinting `use "xchacha20"` | Negative | macros,encryption | None | Pinned: `macros/tests/compile_fail/t035_aes256_gcm_not_implemented.rs` (trybuild) | tests/e2e/macro_e2e.rs (trybuild) |
| ENC-22 | Ciphertext injected via environment variable (`enc:<base64>` form) decrypted into SecretString by config_injector | Normal | security,encryption | None | Inline src tests in `src/security/config_injector.rs` | tests/e2e/encryption_e2e.rs |
| ENC-23 | Illegal algorithm name `encrypt = "rot13"` → macro-expansion compile error (trybuild-level acceptance, hinting the supported list) | Negative | macros,encryption | None | Pinned: `macros/tests/compile_fail/` (encrypt_rot13 and 4 more cases, driven by `compile_fail.rs`) | tests/e2e/macro_e2e.rs (trybuild) |

### 2.6 Key Management (KEY, 13 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| KEY-01 | `KeyBundle::generate` produces a random 32-byte key + metadata | Normal | key | None | Inline src tests in `src/key/mod.rs` | tests/e2e/key_e2e.rs |
| KEY-02 | `KeyBundle::get_plaintext_key(master_key)` recovers the plaintext key with the correct master; wrong master → `ConfigError` | Normal/Negative | key,encryption | None | Inline src key tests | tests/e2e/key_e2e.rs |
| KEY-03 | `KeyRing::rotate` creates a new version and switches the default version | Normal | key | None | Inline src tests in `src/key/mod.rs` rotate group | tests/e2e/key_e2e.rs |
| KEY-04 | `get_key_by_version(nonexistent version)` → None; after `deactivate_version` it is no longer found | Boundary | key | None | Inline src key tests | tests/e2e/key_e2e.rs |
| KEY-05 | After `add_secondary_key`, multiple versions coexist and the old version can still decrypt | Normal | key | None | Inline src key tests | tests/e2e/key_e2e.rs |
| KEY-06 | `KeyRotationSchedule::is_rotation_due/days_until_rotation/update_after_rotation` (due/not-due branches) | Normal/Boundary | key | None | Inline src key tests | tests/e2e/key_e2e.rs |
| KEY-07 | `KeyManager::initialize/generate_key` initialization and random generation | Normal | key | None | Inline src tests in `src/key/manager.rs` | tests/e2e/key_e2e.rs |
| KEY-08 | `create_key_ring/rotate_key` full flow; `get_rotation_status` reflects state | Normal | key | None | Inline src manager tests | tests/e2e/key_e2e.rs |
| KEY-09 | `get_key_info(nonexistent key_id)` → error (`KeyRotationFailed` family, see `tests/core/error.rs::test_key_rotation_failed_error`) | Negative | key | None | Inline src manager tests | tests/e2e/key_e2e.rs |
| KEY-10 | `plan_rotation/cleanup_old_keys(keep_n)`: retained version count correct after cleanup, default key not cleaned up | Boundary | key | None | Inline src manager tests | tests/e2e/key_e2e.rs |
| KEY-11 | After `deprecate_version`, get_key_by_version still fetches the key but marked deprecated | Boundary | key | None | Inline src manager tests | tests/e2e/key_e2e.rs |
| KEY-12 | `set_default_key_id(illegal id)` → error | Negative | key | None | Inline src manager tests | tests/e2e/key_e2e.rs |
| KEY-13 | `KeyMetadata::is_expired/is_active` time edges (same day / instant of expiry) | Boundary | key | None | Inline src tests in `src/key/mod.rs` | tests/e2e/key_e2e.rs |

### 2.7 Security Rules and Injection (SEC, 18 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| SEC-01 | `EnvSecurityValidator` accepts valid env names/values via `validate_env_name/validate_env_value` | Normal | security | None | Inline src tests in `src/security/input_validation.rs` | tests/e2e/security_rules_e2e.rs |
| SEC-02 | Env name containing injection characters (spaces/`=`/newlines/shell metacharacters) → `EnvSecurityError` | Negative | security | None | Inline src input_validation tests | tests/e2e/security_rules_e2e.rs |
| SEC-03 | Env name/value beyond `EnvironmentValidationConfig` limits → rejected | Negative | security | None | Inline src input_validation tests (length branches) | tests/e2e/security_rules_e2e.rs |
| SEC-04 | `strict()` vs `lenient()` reach different verdicts on the same input (e.g. encrypted values / special-mode allowances differ) | Boundary | security | None | Inline src input_validation tests | tests/e2e/security_rules_e2e.rs |
| SEC-05 | `sanitize_for_logging`: sensitive values output as masked | Normal | security | None | Inline src input_validation tests | tests/e2e/security_rules_e2e.rs |
| SEC-06 | Error message redaction: `ConfersError` display chain contains no key/password fragments | Normal | security | None | `tests/security/security.rs::test_error_sanitization`; `tests/core/error.rs::test_error_sanitized_chain/test_error_user_message_formatting` | tests/e2e/security_rules_e2e.rs |
| SEC-07 | `SecurityValidatorRegistry` default rule set validates a complete server config | Normal | security-rules | None | `tests/security/security.rs::test_registry_with_defaults_validates_all` | tests/e2e/security_rules_e2e.rs |
| SEC-08 | One bad config (weak JWT secret + wrong CORS + private-network SSRF) yields multiple `SecurityViolation`s in one pass | Negative | security-rules | None | `tests/security/security.rs::test_registry_detects_multiple_violations` | tests/e2e/security_rules_e2e.rs |
| SEC-09 | Clean config passes entirely, `SecurityReport` has no violation | Normal | security-rules | None | `tests/security/security.rs::test_registry_clean_config_passes` | tests/e2e/security_rules_e2e.rs |
| SEC-10 | fail_on_warning mode: warning-level violations escalate to failure | Boundary | security-rules | None | `tests/security/security.rs::test_fail_on_warning_mode` | tests/e2e/security_rules_e2e.rs |
| SEC-11 | Custom `SecurityValidator` registered and invoked by the registry | Normal | security-rules | None | `tests/security/security.rs::test_custom_validator_registration` | tests/e2e/security_rules_e2e.rs |
| SEC-12 | `CorsValidator`: wildcard origin + credentials combination judged as violation | Negative | security-rules | None | Inline src tests in `src/security/rules/cors.rs` | tests/e2e/security_rules_e2e.rs |
| SEC-13 | `SsrfValidator`: private/link-local addresses (127.0.0.1/10.x/169.254.x) rejected, combined with `remote::is_ip_blocked` | Negative | security-rules,remote | None | Inline src in `src/security/rules/ssrf.rs`, `src/remote/poll.rs::is_ip_blocked` tests | tests/e2e/security_rules_e2e.rs |
| SEC-14 | `JwtSecretValidator`: short/common weak secrets judged as violation | Negative | security-rules | None | Inline src tests in `src/security/rules/jwt.rs` | tests/e2e/security_rules_e2e.rs |
| SEC-15 | `TlsConfigValidator`: disabled verification/skipped verify judged as violation | Negative | security-rules | None | Inline src tests in `src/security/rules/tls.rs` | tests/e2e/security_rules_e2e.rs |
| SEC-16 | `ViolationSeverity` ordering and `SecurityReport` aggregation (critical first) | Boundary | security-rules | None | Inline src rules/mod tests | tests/e2e/security_rules_e2e.rs |
| SEC-17 | `config_injector`: injects external env/secrets into the config tree under a given prefix | Normal | security | None | Inline src tests in `src/security/config_injector.rs` | tests/e2e/security_rules_e2e.rs |
| SEC-18 | Sensitive key prefix recognition (password/secret/token/api_key pattern table in `patterns.rs` + `prefix.rs`) auto-marks sensitive | Boundary | security | None | Inline src tests in `src/security/prefix.rs` | tests/e2e/security_rules_e2e.rs |

### 2.8 Audit (AUD, 11 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| AUD-01 | `AuditWriter::new()` disabled by default: write produces no file and no error | Normal | audit | Local file | `tests/security/audit.rs::test_audit_writer_default` | tests/e2e/audit_e2e.rs |
| AUD-02 | Built via `AuditWriterBuilder::enabled(true).log_dir(dir)`, events are writable | Normal | audit | Local file | `tests/security/audit.rs::test_audit_writer_builder` | tests/e2e/audit_e2e.rs |
| AUD-03 | best-effort mode: `log_load/log_key_access` events persist to the log file | Normal | audit | Local file | `tests/security/audit.rs::test_audit_best_effort_event_persists_to_file/test_audit_log_load/test_audit_log_key_access` | tests/e2e/audit_e2e.rs |
| AUD-04 | durable mode: events persist and failures raise errors | Normal | audit | Local file | `tests/security/audit.rs::test_audit_durable_event_persists_to_file` | tests/e2e/audit_e2e.rs |
| AUD-05 | durable + invalid log_dir → error returned (fail loud) | Negative | audit | Local file | `tests/security/audit.rs::test_audit_no_log_dir_returns_error_for_durable` | tests/e2e/audit_e2e.rs |
| AUD-06 | `log_decrypt(field, success/failure)` output redacted (no plaintext field values) | Normal | audit | Local file | `tests/security/audit.rs::test_audit_log_decrypt_sanitizes` | tests/e2e/audit_e2e.rs |
| AUD-07 | `log_key_rotation(old_ver, new_ver)` event lands on disk | Normal | audit,key | Local file | `tests/security/audit.rs::test_audit_log_key_rotation` | tests/e2e/audit_e2e.rs |
| AUD-08 | `AuditLevel::for_event` event→level mapping (load/access/decrypt/rotation) | Boundary | audit | None | `tests/security/audit.rs::test_audit_level_for_event` | tests/e2e/audit_e2e.rs |
| AUD-09 | `enabled(false)` explicit disable: every log_* call is a no-op | Boundary | audit | Local file | `tests/security/audit.rs::test_audit_disabled` | tests/e2e/audit_e2e.rs |
| AUD-10 | `AuditConfig/AuditConfigBuilder` config surface (defaults/chaining) | Normal | audit | None | `tests/security/audit.rs::test_audit_config_default/test_audit_config_builder` | tests/e2e/audit_e2e.rs |
| AUD-11 | Multi-threaded concurrent writes to the same log file: file line count = writes (no interleaved corruption) | Boundary | audit | Local file | None → to add | tests/e2e/audit_e2e.rs |

### 2.9 File Hot Reload (WAT, 18 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| WAT-01 | `FsWatcher` create/start/stop state machine (is_running transitions) | Normal | watch | Local file | `tests/watcher/watcher.rs::test_fs_watcher_creation/test_fs_watcher_stop` | tests/e2e/watch_e2e.rs |
| WAT-02 | Modifying a watched file's content → change received on the event channel | Normal | watch | Local file | `tests/watcher/watcher.rs::test_fs_watcher_file_modification_detection` | tests/e2e/watch_e2e.rs |
| WAT-03 | Creating a watched file → creation event detected | Normal | watch | Local file | `tests/watcher/watcher.rs::test_fs_watcher_file_creation_detection` | tests/e2e/watch_e2e.rs |
| WAT-04 | Deleting a watched file → deletion event detected and watcher does not panic (subsequent reload goes down the error path) | Negative | watch | Local file | `tests/watcher/watcher.rs::test_fs_watcher_file_deletion_detection` | tests/e2e/watch_e2e.rs |
| WAT-05 | Watching a nonexistent path → error at startup | Negative | watch | Local file | `tests/watcher/watcher.rs::test_fs_watcher_nonexistent_path` | tests/e2e/watch_e2e.rs |
| WAT-06 | `MultiFsWatcher` watches multiple files; each modification delivers an event | Normal | watch | Local file | `tests/watcher/watcher.rs::test_multi_fs_watcher_multiple_files/test_multi_fs_watcher_recv_detects_modification` | tests/e2e/watch_e2e.rs |
| WAT-07 | `MultiFsWatcher` with empty paths → startup behavior explicit (empty set/error, pinned per implementation) | Boundary | watch | Local file | `tests/watcher/watcher.rs::test_multi_fs_watcher_empty_paths` | tests/e2e/watch_e2e.rs |
| WAT-08 | `MultiFsWatcher` containing a nonexistent path → explicit error | Negative | watch | Local file | `tests/watcher/watcher.rs::test_multi_fs_watcher_nonexistent_paths` | tests/e2e/watch_e2e.rs |
| WAT-09 | After `WatcherGuard` drop, the watcher stops (background thread exits) | Normal | watch | Local file | `tests/watcher/watcher.rs::test_watcher_guard_drop_stops_watcher/test_watcher_guard_start_stop/test_watcher_guard_shutdown` | tests/e2e/watch_e2e.rs |
| WAT-10 | `AdaptiveDebouncer`: repeated changes within the window processed once, released after the window | Boundary | watch | Local file | `tests/watcher/watcher.rs::test_adaptive_debouncer_*` (6 cases) | tests/e2e/watch_e2e.rs |
| WAT-11 | `min_reload_interval`: when two changes are closer than the threshold, the second does not trigger a reload | Boundary | watch | Local file | `tests/watcher/watcher.rs::test_watcher_config_with_min_reload_interval` (config surface); behavior-level → to add | tests/e2e/watch_e2e.rs |
| WAT-12 | Consecutive failures reaching `max_consecutive_failures` → enters the `failure_pause` pause window | Negative | watch | Local file | Config surface: `tests/watcher/watcher.rs`; behavior-level: `tests/e2e/watch_e2e.rs::wat12_consecutive_failures_trigger_failure_pause` (executed via the library's `ReloadFailurePolicy`: count → pause → resume at expiry) | tests/e2e/watch_e2e.rs |
| WAT-13 | `rollback_on_validation_failure=true`: reload parse failure rolls back to the last good config | Negative | watch,validation | Local file | `tests/watcher/watcher.rs::test_watcher_config_with_rollback` (config surface); behavior-level → to add | tests/e2e/watch_e2e.rs |
| WAT-14 | File without read permission → permission error reported and watcher does not crash | Negative | watch | Local file | `tests/watcher/watcher.rs::test_fs_watcher_permission_handling` | tests/e2e/watch_e2e.rs |
| WAT-15 | `WatcherConfig` defaults + full builder chain (debounce/min_interval/max_failures/pause/rollback) | Normal | watch | None | `tests/watcher/watcher.rs::test_watcher_config_default/new/builder/builder_partial/builder_chaining/validation/clone/debug` | tests/e2e/watch_e2e.rs |
| WAT-16 | End-to-end hot reload: edit TOML → reload → `DynamicField`/config reads the new value (mirrors the hot_reload example) | Normal | watch,dynamic | Local file | examples/hot_reload; tests-level → to add the full closed loop | tests/e2e/watch_e2e.rs |
| WAT-17 | Editor atomic replacement (write temp file + rename over) triggers exactly one event | Boundary | watch | Local file | None → to add (platform-specific, Linux inotify) | tests/e2e/watch_e2e.rs |
| WAT-18 | Ten rapid consecutive writes (10ms apart) → at most 1-2 reloads after debounce (count-cap assertion) | Boundary | watch | Local file | None → to add (debounce behavior assertion) | tests/e2e/watch_e2e.rs |

### 2.10 Progressive Reload (PGR, 12 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| PGR-01 | Immediate strategy: healthy check commits the new config | Normal | progressive-reload | None | `tests/core/progressive.rs::test_immediate_reload_commits` | tests/e2e/progressive_e2e.rs |
| PGR-02 | Immediate commit atomicity: `current()` is always the complete old value or the complete new value (arc-swap) | Boundary | progressive-reload | None | `tests/core/progressive.rs::test_immediate_reload_atomic` | tests/e2e/progressive_e2e.rs |
| PGR-03 | Canary strategy: health check Healthy → commit | Normal | progressive-reload | None | `tests/core/progressive.rs::test_canary_reload_commits_when_healthy` | tests/e2e/progressive_e2e.rs |
| PGR-04 | Canary: Critical → roll back to the old config | Negative | progressive-reload | None | `tests/core/progressive.rs::test_canary_reload_rollback_on_critical` | tests/e2e/progressive_e2e.rs |
| PGR-05 | Canary: Degraded → keep advancing without rollback | Boundary | progressive-reload | None | `tests/core/progressive.rs::test_canary_reload_continues_on_degraded` | tests/e2e/progressive_e2e.rs |
| PGR-06 | Canary without health_check: commit allowed by default | Boundary | progressive-reload | None | `tests/core/progressive.rs::test_canary_reload_without_health_check` | tests/e2e/progressive_e2e.rs |
| PGR-07 | Linear strategy: commits step by step once step dependencies are ready | Normal | progressive-reload | None | `tests/core/progressive.rs::test_linear_reload_commits_after_steps` | tests/e2e/progressive_e2e.rs |
| PGR-08 | Custom `ReloadHealthCheck` rejects an invalid new config (real validation function) | Negative | progressive-reload,validation | None | `tests/core/progressive.rs::test_real_health_check_validates_config/test_real_health_check_critical_on_invalid` | tests/e2e/progressive_e2e.rs |
| PGR-09 | `ProgressiveReloader` Clone semantics and `current()` returning an Arc snapshot | Normal | progressive-reload | None | `tests/core/progressive.rs::test_progressive_reloader_is_clone/test_current_returns_arc` | tests/e2e/progressive_e2e.rs |
| PGR-10 | Builder: initial/strategy/health_check/with_dependencies chained construction | Normal | progressive-reload | None | `tests/core/progressive.rs::test_builder_default/test_with_dependencies` | tests/e2e/progressive_e2e.rs |
| PGR-11 | `ReloadOutcome`/`HealthStatus` variants complete (Committed/RolledBack etc. × Healthy/Degraded/Critical) | Boundary | progressive-reload | None | `tests/core/progressive.rs::test_reload_outcome_variants/test_health_status_variants` | tests/e2e/progressive_e2e.rs |
| PGR-12 | ConfigProvider adaptation: `config.health_check()` Degraded reporting (including the lifecycle registration path) | Boundary | progressive-reload | None | `tests/core/progressive.rs::test_config_provider_implementation/test_config_health_check_degraded` | tests/e2e/progressive_e2e.rs |

### 2.11 Dynamic Fields (DYN, 14 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| DYN-01 | `DynamicField` initial value read (get/get_ref, strings and numbers) | Normal | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_get_returns_initial/test_dynamic_field_get_returns_initial_string/test_dynamic_field_get_ref` | tests/e2e/dynamic_e2e.rs |
| DYN-02 | `update()` applies the new value and fires callbacks (callbacks receive the new value) | Normal | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_update/test_dynamic_field_callback_registration` | tests/e2e/dynamic_e2e.rs |
| DYN-03 | Multiple callbacks registered on the same field are all invoked | Normal | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_multiple_callbacks/test_dynamic_field_callback_count_tracking` | tests/e2e/dynamic_e2e.rs |
| DYN-04 | update succeeds silently when no callbacks are registered | Boundary | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_no_callbacks` | tests/e2e/dynamic_e2e.rs |
| DYN-05 | `CallbackGuard` drop auto-deregisters the callback | Normal | dynamic | None | `tests/core/dynamic.rs::test_callback_guard_deregisters_on_drop/test_callback_guard_drops_on_scope_exit` | tests/e2e/dynamic_e2e.rs |
| DYN-06 | Multiple guards deregister independently without affecting each other | Boundary | dynamic | None | `tests/core/dynamic.rs::test_multiple_callback_guards` | tests/e2e/dynamic_e2e.rs |
| DYN-07 | Async callbacks are scheduled and executed | Normal | dynamic,tokio | None | `tests/core/dynamic.rs::test_dynamic_field_async_callback` | tests/e2e/dynamic_e2e.rs |
| DYN-08 | Complex-typed field (nested struct) updated dynamically | Normal | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_complex_type` | tests/e2e/dynamic_e2e.rs |
| DYN-09 | `DynamicFieldBuilder` chained construction and Default impl | Normal | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_builder/test_dynamic_field_builder_default/test_dynamic_field_default` | tests/e2e/dynamic_e2e.rs |
| DYN-10 | `FieldWatcher` (dynamic+watch): fires only when the field value changes | Normal | dynamic,watch | None | `tests/core/dynamic.rs::test_field_watcher_changed_for` | tests/e2e/dynamic_e2e.rs |
| DYN-11 | `FieldWatcher`: no callback when the field is unchanged | Boundary | dynamic,watch | None | `tests/core/dynamic.rs::test_field_watcher_no_trigger_if_field_unchanged` | tests/e2e/dynamic_e2e.rs |
| DYN-12 | Multi-threaded concurrent update + read without data races (locking strategy verified) | Boundary | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_multithread_callbacks` | tests/e2e/dynamic_e2e.rs |
| DYN-13 | Integrated with a real `ConfigConnector`: after config set, the dynamic field reflects it | Normal | dynamic | None | `tests/core/dynamic.rs::test_dynamic_field_with_real_config/test_callback_with_real_config/test_real_config_provider` | tests/e2e/dynamic_e2e.rs |
| DYN-14 | Derive macro `#[config(dynamic = true)]` field generates a handle; after reload the handle reads the new value | Normal | dynamic,macros | Local file | Inline src in `macros/src/codegen` + examples/dynamic_fields; integration level → to add | tests/e2e/dynamic_e2e.rs |

### 2.12 Feature Toggle (TGL, 6 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| TGL-01 | register→enable→disable→toggle→is_enabled full lifecycle states correct | Normal | feature-toggle | None | `tests/core/toggle.rs::test_toggle_lifecycle` | tests/e2e/toggle_e2e.rs |
| TGL-02 | enable/disable/toggle/is_enabled on an unregistered toggle → returns false (no panic) | Negative | feature-toggle | None | `tests/core/toggle.rs::test_toggle_lifecycle` (partial) + inline src toggle tests → explicit scenario needed | tests/e2e/toggle_e2e.rs |
| TGL-03 | `load_from_config(provider, "features")`: batch-initializes toggles from the config tree | Normal | feature-toggle | Local file | `tests/core/toggle.rs::test_load_from_config_integration` | tests/e2e/toggle_e2e.rs |
| TGL-04 | Multi-threaded concurrent enable/disable/toggle converges to a consistent final state (dashmap concurrency safety) | Boundary | feature-toggle | None | `tests/core/toggle.rs::test_concurrent_toggle_operations` | tests/e2e/toggle_e2e.rs |
| TGL-05 | `list()` returns all FeatureInfo (name/description/state); len/is_empty consistent | Normal | feature-toggle | None | `tests/core/toggle.rs::test_list_returns_all_toggles` | tests/e2e/toggle_e2e.rs |
| TGL-06 | Toggle gates dynamic fields/callbacks on and off (dynamic+toggle combination entry) | Boundary | feature-toggle,dynamic | None | None → to add | tests/e2e/combo_e2e.rs (CMP-03) |

### 2.13 Version Migration (MIG, 12 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| MIG-01 | `Versioned` trait: version() returns the declared version; different structs carry different versions | Normal | migration | None | `tests/core/migration.rs::test_versioned_trait_implementation/test_versioned_different_versions` | tests/e2e/migration_e2e.rs |
| MIG-02 | `MigrationRegistry::register(v1,v2,fn)` registers and returns self (builder chain) | Normal | migration | None | `tests/core/migration.rs::test_registry_register/test_registry_register_returns_self/test_registry_builder_pattern/test_registry_with_migrations` | tests/e2e/migration_e2e.rs |
| MIG-03 | Direct migration v1→v2 transforms data correctly | Normal | migration | None | `tests/core/migration.rs::test_migrate_direct` | tests/e2e/migration_e2e.rs |
| MIG-04 | Chained migration v1→v3 (via v2) executes in order | Normal | migration | None | `tests/core/migration.rs::test_migrate_chain/test_precompute_paths_chain` | tests/e2e/migration_e2e.rs |
| MIG-05 | Same-version migration → no-op returned directly | Boundary | migration | None | `tests/core/migration.rs::test_migrate_same_version` | tests/e2e/migration_e2e.rs |
| MIG-06 | No migration path (v1→v9 unregistered) → `MigrationError` (2600 version-mismatch family) | Negative | migration | None | `tests/core/migration.rs::test_migrate_no_path`; `tests/core/error.rs::test_migration_error` | tests/e2e/migration_e2e.rs |
| MIG-07 | Error inside a migration fn → error propagates to the caller, no half-built config produced | Negative | migration | None | `tests/core/migration.rs::test_migrate_failure` | tests/e2e/migration_e2e.rs |
| MIG-08 | `precompute_paths` picks the right route on a complex graph (multi-branch); direct path preferred over chain | Boundary | migration | None | `tests/core/migration.rs::test_precompute_paths_direct/complex_graph/direct_preferred_over_chain/no_path` | tests/e2e/migration_e2e.rs |
| MIG-09 | `MigrationOnReload` three states (Skip/Always/IfChanged) semantics and default value | Boundary | migration | None | `tests/core/migration.rs::test_migration_on_reload_variants/default/clone/debug` | tests/e2e/migration_e2e.rs |
| MIG-10 | Derive macro `ConfigMigration` generates the Versioned impl (version attribute passed through) | Normal | migration,macros | None | `tests/core/derive.rs::test_config_migration_derive_generates_versioned` | tests/e2e/migration_e2e.rs |
| MIG-11 | migration+snapshot combination: after migrating old-version config, the snapshot records new-version content | Boundary | migration,snapshot | Local file | None → to add | tests/e2e/combo_e2e.rs (CMP-02) |
| MIG-12 | migration+watch combination: hot-reloaded file reverted to an old version → auto-migrated then applied | Boundary | migration,watch | Local file | None → to add | tests/e2e/combo_e2e.rs (CMP-05) |

### 2.14 Snapshot (SNP, 10 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| SNP-01 | `SnapshotManager::save` persists in JSON format, filename contains a timestamp | Normal | snapshot | Local file | Inline src tests in `src/impl_/snapshot.rs`; examples/snapshot | tests/e2e/snapshot_e2e.rs |
| SNP-02 | `SnapshotFormat::{Toml,Yaml}` save→load roundtrip for each format | Normal | snapshot | Local file | Inline src snapshot tests (`SnapshotFormat::ext`) | tests/e2e/snapshot_e2e.rs |
| SNP-03 | `list_snapshots` returns `SnapshotInfo` sorted by time | Normal | snapshot | Local file | Inline src snapshot tests | tests/e2e/snapshot_e2e.rs |
| SNP-04 | `load_snapshot(path)` restores an `AnnotatedValue` equivalent to the saved one | Normal | snapshot | Local file | Inline src snapshot tests | tests/e2e/snapshot_e2e.rs |
| SNP-05 | `prune_old_snapshots` cleans expired snapshots and returns the count | Normal | snapshot | Local file | Inline src snapshot tests | tests/e2e/snapshot_e2e.rs |
| SNP-06 | list/prune on a nonexistent snapshot directory → empty list/explicit handling (no panic) | Boundary | snapshot | Local file | `tests/cli/commands.rs::test_snapshot_list_empty` (CLI surface); library surface → to add | tests/e2e/snapshot_e2e.rs |
| SNP-07 | `load_snapshot(nonexistent file)` → error | Negative | snapshot | Local file | Inline src snapshot tests → explicit scenario needed | tests/e2e/snapshot_e2e.rs |
| SNP-08 | Corrupted snapshot (garbage content written) load → parse error | Negative | snapshot | Local file | None → to add | tests/e2e/snapshot_e2e.rs |
| SNP-09 | `SnapshotConfig` max-count cap: oldest snapshot rolls off when exceeded (after saving N+1, the oldest disappears) | Boundary | snapshot | Local file | Inline src snapshot tests → to add integration | tests/e2e/snapshot_e2e.rs |
| SNP-10 | `ConfigBuilder::with_snapshot` automatic snapshot at build end-to-end (build → snapshot file exists → replayable) | Normal | snapshot | Local file | Inline src builder with_snapshot tests → to add integration | tests/e2e/snapshot_e2e.rs |

### 2.15 Modules / Config Groups (MOD, 11 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| MOD-01 | `ModuleConfig::new(name, [(profile,path)], default)` registers multiple profiles; profiles/has_profile/get_profile correct | Normal | modules | Local file | `tests/core/modules.rs::test_module_config_new/get_profile/has_profile/profiles` | tests/e2e/modules_e2e.rs |
| MOD-02 | default profile inference: use default when present, otherwise take the first | Boundary | modules | Local file | `tests/core/modules.rs::test_module_config_default_profile_with_default/without_default` | tests/e2e/modules_e2e.rs |
| MOD-03 | `set_active_profile`: legal switch succeeds; illegal profile/group → error | Negative | modules | Local file | `tests/core/modules.rs::test_set_active_profile/nonexistent_profile/nonexistent_group` | tests/e2e/modules_e2e.rs |
| MOD-04 | `load_module` loads toml/json profile files successfully | Normal | modules | Local file | `tests/core/modules.rs::test_load_module_success_toml/test_load_module_success_json` | tests/e2e/modules_e2e.rs |
| MOD-05 | `load_module(group does not exist)` → `ModuleNotFound` error | Negative | modules | Local file | `tests/core/modules.rs::test_load_module_not_found_group`; `tests/core/error.rs::test_module_not_found_error` | tests/e2e/modules_e2e.rs |
| MOD-06 | `load_module(profile does not exist)` → error | Negative | modules | Local file | `tests/core/modules.rs::test_load_module_not_found_profile` | tests/e2e/modules_e2e.rs |
| MOD-07 | `load_module(file does not exist)` → file error rather than panic | Negative | modules | Local file | `tests/core/modules.rs::test_load_module_file_not_found` | tests/e2e/modules_e2e.rs |
| MOD-08 | `load_active`: loads per the currently active profile; none active → error | Normal/Negative | modules | Local file | `tests/core/modules.rs::test_load_active_success/test_load_active_not_found` | tests/e2e/modules_e2e.rs |
| MOD-09 | `ModuleRegistry` multi-group registration, list_groups, chaining, get/contains/len | Normal | modules | Local file | `tests/core/modules.rs::test_register_group/register_multiple_groups/list_groups/registry_chaining/multiple_groups_load/get_module_config/registry_with_capacity` | tests/e2e/modules_e2e.rs |
| MOD-10 | Derive macro `ConfigModules` generates a registry (module_group attribute) | Normal | modules,macros | None | `tests/core/derive.rs::test_config_modules_derive_generates_registry` | tests/e2e/modules_e2e.rs |
| MOD-11 | profile_env (APP_ENV) drives active-profile switching (struct attributes profile/profile_env end-to-end) | Boundary | modules,macros | Local file | Inline src in `macros/src/codegen/modules.rs`; integration level → to add | tests/e2e/modules_e2e.rs |

### 2.16 Context Awareness (CTX, 9 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| CTX-01 | `EvaluationContext` default/with_key/with_attributes/clone/debug | Normal | context-aware | None | `tests/core/context.rs::test_evaluation_context_default/with_key/with_attributes/clone/debug` | tests/e2e/context_e2e.rs |
| CTX-02 | `ContextRule` predicate matches → `ContextAwareField` returns the rule value | Normal | context-aware | None | `tests/core/context.rs::test_context_aware_field_with_matching_predicate` | tests/e2e/context_e2e.rs |
| CTX-03 | No rule matches → default value returned | Boundary | context-aware | None | `tests/core/context.rs::test_context_aware_field_no_match_returns_default` | tests/e2e/context_e2e.rs |
| CTX-04 | Multiple rules evaluated in order: first match returns (priority order pinned) | Boundary | context-aware | None | `tests/core/context.rs::test_context_aware_field_multiple_rules` | tests/e2e/context_e2e.rs |
| CTX-05 | `evaluate` returns reference semantics (`evaluate_returns_reference`) | Normal | context-aware | None | `tests/core/context.rs::test_context_aware_field_evaluate_returns_reference` | tests/e2e/context_e2e.rs |
| CTX-06 | environment/region dimension context (dev/prod, cn/us) value switching | Normal | context-aware | None | `tests/core/context.rs::test_evaluation_context_environment/region` | tests/e2e/context_e2e.rs |
| CTX-07 | `test_upload_limit_use_case` end-to-end: per-region upload limit | Normal | context-aware | None | `tests/core/context.rs::test_upload_limit_use_case` | tests/e2e/context_e2e.rs |
| CTX-08 | `ContextAwareField` Send+Sync (cross-thread evaluation) | Boundary | context-aware | None | `tests/core/context.rs::test_context_aware_field_send_sync` | tests/e2e/context_e2e.rs |
| CTX-09 | context-aware + dynamic combination: context change triggers hot value switch | Boundary | context-aware,dynamic | None | None → to add | tests/e2e/combo_e2e.rs (CMP-17) |

### 2.17 HTTP Polled Remote Source (REM, 11 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| REM-01 | `HttpPolledSourceBuilder` without url → `build()` errors | Negative | remote | None | `tests/remote/remote.rs::test_builder_requires_url` | tests/e2e/remote_e2e.rs |
| REM-02 | Full builder chain: url/interval/format/timeout/allowed_domain(s) | Normal | remote | None | `tests/remote/remote.rs::test_builder_accepts_url/custom_interval/with_format/with_timeout/builder_pattern` | tests/e2e/remote_e2e.rs |
| REM-03 | Default poll interval value pinned (`test_default_poll_interval`) | Boundary | remote | None | `tests/remote/remote.rs::test_default_poll_interval` | tests/e2e/remote_e2e.rs |
| REM-04 | Various URL forms (host:port, https://, with path) accepted | Normal | remote | None | `tests/remote/remote.rs::test_various_url_formats` | tests/e2e/remote_e2e.rs |
| REM-05 | SSRF protection: plaintext http URL rejected | Negative | remote | None | `tests/remote/remote.rs::test_http_url_rejected` | tests/e2e/remote_e2e.rs |
| REM-06 | SSRF protection: private-network IP targets rejected (with `is_ip_blocked`) | Negative | remote | None | `tests/remote/remote.rs::test_private_ip_rejected` | tests/e2e/remote_e2e.rs |
| REM-07 | Illegal/empty URL → build error | Negative | remote | None | `tests/remote/remote.rs::test_invalid_url_rejected/test_empty_url_rejected` | tests/e2e/remote_e2e.rs |
| REM-08 | `PolledSource` trait object is boxable (dyn compatible) | Normal | remote | None | `tests/remote/remote.rs::test_trait_object` | tests/e2e/remote_e2e.rs |
| REM-09 | Unreachable poll target → error/retry path; after circuit_breaker_threshold trips it fails fast, backing off to max_delay | Negative | remote | None (unreachable address) | Inline src tests in `src/remote/circuit_breaker.rs`; poll level → to add | tests/e2e/remote_e2e.rs |
| REM-10 | Circuit-breaker half-open recovery: source usable again after the fault clears (real HTTP endpoint; a compose service may substitute) | Boundary | remote | Local file (a Consul impersonating the HTTP endpoint also works) | None → to add | tests/e2e/remote_e2e.rs |
| REM-11 | Polled content changes → new config takes effect (end-to-end poll loop) | Normal | remote | Consul(8500) (as HTTP KV endpoint) | None → to add | tests/e2e/remote_e2e.rs |

### 2.18 etcd (ETC, 10 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| ETC-01 | `EtcdSourceBuilder::endpoint("127.0.0.1:2379")` connects to a real etcd | Normal | etcd | etcd(2379) | `tests/remote/etcd.rs::test_etcd_source_connect` (runs only with the service, guarded by `tests/common.rs::is_service_available`) | tests/e2e/remote_e2e.rs |
| ETC-02 | Username/password authenticated connection | Normal | etcd | etcd(2379) | `tests/remote/etcd.rs::test_etcd_source_with_auth` | tests/e2e/remote_e2e.rs |
| ETC-03 | Writing TOML KV → source parses into a config tree; JSON form likewise | Normal | etcd,toml,json | etcd(2379) | `tests/remote/etcd.rs::test_etcd_source_parse_config/test_etcd_source_json_config` | tests/e2e/remote_e2e.rs |
| ETC-04 | Auth credentials read from environment variables (`test_etcd_auth_env_var` semantics) | Normal | etcd | etcd(2379) | `tests/remote/remote.rs::test_etcd_auth_env_var` | tests/e2e/remote_e2e.rs |
| ETC-05 | Illegal endpoint string → construction error | Negative | etcd | None | `tests/remote/remote.rs::test_etcd_invalid_endpoint` | tests/e2e/remote_e2e.rs |
| ETC-06 | etcd unreachable (127.0.0.1:1) → connection error | Negative | etcd | None (unreachable address) | `tests/remote/remote.rs::test_etcd_connection_failure` | tests/e2e/remote_e2e.rs |
| ETC-07 | TLS connection configuration (tls branch construction) | Boundary | etcd | etcd(2379) | `tests/remote/remote.rs::test_etcd_tls_connection/test_etcd_builder_tls` | tests/e2e/remote_e2e.rs |
| ETC-08 | Builder chain endpoint/auth/prefix/interval/tls chaining | Normal | etcd | None | `tests/remote/remote.rs::test_etcd_builder_*` (6 cases) | tests/e2e/remote_e2e.rs |
| ETC-09 | End-to-end: write KV → poll read → change KV → read new value (remote hot-reload semantics) | Normal | etcd,remote | etcd(2379) | None → to add | tests/e2e/remote_e2e.rs |
| ETC-10 | Concurrent writes of multiple keys then polled read: prefix tree complete (no lost keys / half-written state) | Boundary | etcd | etcd(2379) | None → to add | tests/e2e/remote_e2e.rs |

### 2.19 Consul (CSL, 8 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| CSL-01 | `ConsulSourceBuilder::address("127.0.0.1:8500")` connects to a real Consul | Normal | consul | Consul(8500) | `tests/remote/consul.rs::test_consul_source_connect` (service guard) | tests/e2e/remote_e2e.rs |
| CSL-02 | Accessing protected KV with a token (dev-mode default token semantics pinned) | Normal | consul | Consul(8500) | `tests/remote/consul.rs::test_consul_source_with_token` | tests/e2e/remote_e2e.rs |
| CSL-03 | KV tree under a prefix parsed into config | Normal | consul | Consul(8500) | `tests/remote/consul.rs::test_consul_source_parse_config` | tests/e2e/remote_e2e.rs |
| CSL-04 | Consul unreachable → connection error | Negative | consul | None (unreachable address) | `tests/remote/remote.rs::test_consul_connection_failure` | tests/e2e/remote_e2e.rs |
| CSL-05 | tls_skip_verify (debug) branch construction | Boundary | consul | Consul(8500) | `tests/remote/remote.rs::test_consul_tls_config/test_consul_builder_tls_skip_verify_debug` | tests/e2e/remote_e2e.rs |
| CSL-06 | Builder chain address/token/prefix/interval | Normal | consul | None | `tests/remote/remote.rs::test_consul_builder_*` (5 cases) | tests/e2e/remote_e2e.rs |
| CSL-07 | End-to-end: Consul write KV → poll reads new value → delete KV → empty config handled | Normal | consul,remote | Consul(8500) | None → to add | tests/e2e/remote_e2e.rs |
| CSL-08 | DoS protection: `max_response_bytes`/`max_kv_entries` over-limit rejected | Boundary | consul | Consul(8500) | Inline src consul limits tests (demonstrated in examples/remote_consul); integration level → to add | tests/e2e/remote_e2e.rs |

### 2.20 Config Bus — In-Process (BUS, 10 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| BUS-01 | `InMemoryBus::new/with_capacity` creation and `subscriber_count` | Normal | config-bus | None | `tests/remote/bus.rs::test_in_memory_bus_new/test_in_memory_bus_with_capacity` | tests/e2e/bus_e2e.rs |
| BUS-02 | `publish` → `subscribe` receives `ConfigChangeEvent` (instance/source/keys/checksum) | Normal | config-bus | None | `tests/remote/bus.rs::test_in_memory_bus_publish_subscribe/test_config_change_event_creation` | tests/e2e/bus_e2e.rs |
| BUS-03 | Multi-subscriber broadcast: one event received by all | Normal | config-bus | None | `tests/remote/bus.rs::test_in_memory_bus_multiple_subscribers` | tests/e2e/bus_e2e.rs |
| BUS-04 | Capacity full (capacity=1 with several rapid publishes): lagging-subscriber behavior explicit (drop-old/block, pinned per implementation), no panic | Boundary | config-bus | None | None → to add | tests/e2e/bus_e2e.rs |
| BUS-05 | `BusBuilder::capacity(n).build()` chained construction | Normal | config-bus | None | `tests/remote/bus.rs::test_bus_builder` | tests/e2e/bus_e2e.rs |
| BUS-06 | `ConfigChangeEvent` serialization roundtrip (serde) | Normal | config-bus | None | `tests/remote/bus.rs::test_event_serialization` | tests/e2e/bus_e2e.rs |
| BUS-07 | `BusEventLimiter`: `try_acquire` rate limiting, `release/reset` semantics | Boundary | config-bus | None | Inline src tests in `src/bus/limiter.rs` | tests/e2e/bus_e2e.rs |
| BUS-08 | `ConfigBuilder::config_bus(Arc<dyn ConfigBus>)` integration: file-change events drive application reload | Normal | config-bus,watch | Local file | None → to add | tests/e2e/combo_e2e.rs (CMP-04) |
| BUS-09 | After `Lifecycle` start/stop, publish/subscribe state explicit (publish after stop errors or is ignored) | Boundary | config-bus | None | lifecycle tests in `tests/remote/bus.rs` (nats/redis start_stop are the real-service versions); InMemory level → to add | tests/e2e/bus_e2e.rs |
| BUS-10 | Publishing does not panic after a subscriber's receiver is dropped; subscriber_count decrements | Boundary | config-bus | None | None → to add | tests/e2e/bus_e2e.rs |

### 2.21 NATS Bus (NAT, 7 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| NAT-01 | `NatsConfigBus::connect("nats://127.0.0.1:4222", subject)` connects to a real NATS | Normal | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_connect/test_nats_bus_connect_direct` (port guard, skipped without the service) | tests/e2e/bus_e2e.rs |
| NAT-02 | Connecting to an unreachable NATS (wrong port) → connection error, no panic | Negative | nats-bus | None (unreachable address) | `tests/remote/bus.rs::test_nats_bus_connect` (failure branch) | tests/e2e/bus_e2e.rs |
| NAT-03 | `NatsBusBuilder` url/subject/stream_name/options → build | Normal | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_url_method/test_nats_bus_subject/test_nats_bus_with_stream_name/test_nats_bus_options` | tests/e2e/bus_e2e.rs |
| NAT-04 | start/stop lifecycle (JetStream stream establishment and cleanup) | Normal | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_start_stop` | tests/e2e/bus_e2e.rs |
| NAT-05 | End-to-end publish/subscribe (cross-instance: two bus instances exchange events) | Normal | nats-bus | NATS(4222) | `tests/remote/bus.rs::test_nats_bus_publish_subscribe` | tests/e2e/bus_e2e.rs |
| NAT-06 | Bus disconnect: stopping the NATS container/cutting the connection during the test → error reported; after recovery, sending and receiving work again | Negative | nats-bus | NATS(4222) | None → to add | tests/e2e/bus_e2e.rs |
| NAT-07 | nats-bus injected into `ConfigBuilder::config_bus`: instance A changes config → instance B receives the event and reloads | Boundary | nats-bus,watch | NATS(4222) | None → to add | tests/e2e/combo_e2e.rs (CMP-16) |

### 2.22 Redis Bus (RDS, 7 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| RDS-01 | `RedisConfigBus::connect("redis://127.0.0.1:16379", channel)` connects to a real Redis | Normal | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_connect/test_redis_bus_default` (port guard) | tests/e2e/bus_e2e.rs |
| RDS-02 | Connecting to an unreachable Redis → connection error | Negative | redis-bus | None (unreachable address) | `tests/remote/bus.rs::test_redis_bus_connect` (failure branch) | tests/e2e/bus_e2e.rs |
| RDS-03 | `RedisBusBuilder` url/channel/retry_wait_ms/error_retry_wait_secs → build (pool_size removed in 0.6.0; MultiplexedConnection multiplexes a single connection) | Normal | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_url_method` | tests/e2e/bus_e2e.rs |
| RDS-04 | start/stop lifecycle | Normal | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_start_stop` | tests/e2e/bus_e2e.rs |
| RDS-05 | End-to-end pub/sub (pub/sub end-to-end event exchange) | Normal | redis-bus | Redis(16379) | `tests/remote/bus.rs::test_redis_bus_publish_subscribe` | tests/e2e/bus_e2e.rs |
| RDS-06 | Multi-task concurrent publishing loses no messages (single multiplexed connection) | Boundary | redis-bus | Redis(16379) | `tests/e2e/bus_e2e.rs::rds06_concurrent_publish_delivers_all` | tests/e2e/bus_e2e.rs |
| RDS-07 | Bus disconnect recovery: flushall/disconnect midway → retried per error_retry_wait_secs, then recovers | Negative | redis-bus | Redis(16379) | None → to add | tests/e2e/bus_e2e.rs |

### 2.23 Schema (SCH, 5 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| SCH-01 | `ConfigSchema` derive generates a JSON Schema: types/required/defaults correct | Normal | schema,macros | None | `tests/core/derive.rs::test_config_schema_derive_generates_json_schema` | tests/e2e/schema_e2e.rs |
| SCH-02 | `TypeScriptGenerator::generate::<T>()` generates TS type declarations; nested structures map to interfaces | Normal | typescript-schema,macros | None | `tests/core/derive.rs::test_config_schema_derive_generates_typescript_type`, inline src tests in `src/impl_/schema.rs` | tests/e2e/schema_e2e.rs |
| SCH-03 | `typescript-schema` includes `json-schema`: enabling the former grants all schema capabilities (`schema` is the old-name compatibility alias) | Boundary | typescript-schema | None | Cargo.toml declaration check + PRS compile matrix | tests/e2e/presets_e2e.rs |
| SCH-04 | Schema output correctness for nested struct/Option/Vec fields | Boundary | schema | None | Inline src schema tests → to add assertions | tests/e2e/schema_e2e.rs |
| SCH-05 | Schema coupled with CLI validate: a bad config validated against schema semantics reports issues | Negative | schema,cli | Local file | `tests/cli/commands.rs::test_validate_command` (heuristic validation) | tests/e2e/cli_e2e.rs |

### 2.24 CLI (CLI, 24 scenarios, binary `confers`, `cargo run --features cli -- …` or assert_cmd form)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| CLI-01 | `inspect -c app.toml` prints a text table with KEY/VALUE/SOURCE/LOCATION | Normal | cli | Local file | `tests/cli/commands.rs::test_inspect_command` | tests/e2e/cli_e2e.rs |
| CLI-02 | `inspect --format json` prints parseable JSON (provenance tree) | Normal | cli | Local file | `tests/cli/commands.rs::test_inspect_json_output` | tests/e2e/cli_e2e.rs |
| CLI-03 | `inspect -k server.port` filters; nonexistent key prints `[NOT FOUND]` and exits 0 | Normal/Boundary | cli | Local file | Inline src `src/cli/mod.rs::test_find_value_by_key_missing`; CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-04 | `inspect --show-conflicts`: overridden keys carry the `*` conflict marker | Boundary | cli | Local file | Inline src `test_print_config_value_conflict_marker_when_priority`; CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-05 | `validate` on a valid config → success + "All validation checks passed" | Normal | cli | Local file | `tests/cli/commands.rs::test_validate_command` | tests/e2e/cli_e2e.rs |
| CLI-06 | `validate --strict` exits non-zero when issues exist | Negative | cli | Local file | Inline src strict-branch test; CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-07 | `validate --format json` prints the `{"valid":…,"issues":[…]}` structure | Normal/Negative | cli | Local file | `tests/cli/commands.rs::test_validate_json_output` | tests/e2e/cli_e2e.rs |
| CLI-08 | `export --format json/toml` prints the merged config | Normal | cli | Local file | `tests/cli/commands.rs::test_export_command_json/test_export_command_toml` | tests/e2e/cli_e2e.rs |
| CLI-09 | `export -o file` writes the file; `-o directory` auto-generates a timestamped filename | Normal | cli | Local file | `tests/cli/commands.rs::test_export_to_file` | tests/e2e/cli_e2e.rs |
| CLI-10 | `export --with-provenance` prints an annotated tree with source/location | Normal | cli | Local file | `tests/cli/commands.rs::test_export_with_provenance` | tests/e2e/cli_e2e.rs |
| CLI-11 | `export --raw` prints the un-redacted warning to stderr | Boundary | cli | Local file | Inline src raw-warning branch; CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-12 | `diff --base a.toml --overlay b.toml` prints a unified diff; `--format json` prints structured output | Normal | cli | Local file | `tests/cli/commands.rs::test_diff_command/test_diff_json_output` | tests/e2e/cli_e2e.rs |
| CLI-13 | `diff` on two identical files → "Configurations are identical" | Boundary | cli | Local file | `tests/cli/commands.rs::test_diff_identical_configs` | tests/e2e/cli_e2e.rs |
| CLI-14 | `diff` with absolute paths → bail message; succeeds after adding `--allow-absolute-paths` | Negative | cli | Local file | Inline src absolute-path bail-branch test; CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-15 | `snapshot list --directory`: two outputs for empty directory / directory with json+toml snapshots | Normal/Boundary | cli | Local file | `tests/cli/commands.rs::test_snapshot_list_empty`; non-empty directory level → to add | tests/e2e/cli_e2e.rs |
| CLI-16 | `snapshot diff --latest 2`: friendly message and exit when fewer than 2 snapshots | Negative | cli | Local file | Inline src `cmd_snapshot_diff` insufficient branch; CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-17 | `snapshot prune --older-than abc` → fail loud: invalid duration error | Negative | cli | Local file | Inline src `cmd_snapshot_prune` parse-failure branch; CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-18 | `--env-file .env` loads variables into the config; missing file → error exit | Normal/Negative | cli,env | Local file | `tests/cli/commands.rs::test_env_file_loading`, inline src `test_load_env_file_not_found` | tests/e2e/cli_e2e.rs |
| CLI-19 | `--env-file` over 1 MiB → "size limit" error | Negative | cli,env | Local file | Inline src `test_load_env_file_rejects_oversized_file` | tests/e2e/cli_e2e.rs |
| CLI-20 | `--env-file` single line over 16 KiB → "line too long" error | Negative | cli,env | Local file | Inline src `test_load_env_file_rejects_oversized_line` | tests/e2e/cli_e2e.rs |
| CLI-21 | `--help`/`--version` prints the subcommand list and version | Normal | cli | None | `tests/cli/commands.rs::test_help_flag/test_version_flag`, `tests/cli/help.rs` (6 cases) | tests/e2e/cli_e2e.rs |
| CLI-22 | `-c nonexistent path`: path skipped (only env participates in the build) without error | Boundary | cli | Local file | Inline src `test_build_config_from_cli_nonexistent_path_skipped` | tests/e2e/cli_e2e.rs |
| CLI-23 | `export --format xml` / `diff --format xml` → "Unsupported format" bail with non-zero exit | Negative | cli | Local file | Inline src unsupported-format branches (export/diff); CLI level → to add | tests/e2e/cli_e2e.rs |
| CLI-24 | inspect truncates strings over 20 chars; UTF-8 multi-byte (Chinese) truncated per character without panic | Boundary | cli | Local file | Inline src `test_format_value_long_string_truncation` etc. | tests/e2e/cli_e2e.rs |

### 2.25 Derive Macros and Attributes (MAC, 18 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| MAC-01 | `#[derive(Config)]` struct without attributes: compiles and generates loading code, isomorphic to `test_simple_config_default` | Normal | macros | None | `tests/core/derive.rs::test_simple_config_default` | tests/e2e/macro_e2e.rs |
| MAC-02 | Field env mapping: `field_name` → `FIELD_NAME` environment variable override | Normal | macros,env | None | `tests/core/derive.rs::test_simple_config_env_mapping` | tests/e2e/macro_e2e.rs |
| MAC-03 | `name` / `name_env` field attributes override default key/env names | Boundary | macros | Local file | Inline src `macros/src/parse.rs::effective_name/effective_env_name` tests; integration level → to add | tests/e2e/macro_e2e.rs |
| MAC-04 | `Option<T>` field missing → None, no MissingField | Boundary | macros | Local file | `tests/core/derive.rs::test_optional_config_default` | tests/e2e/macro_e2e.rs |
| MAC-05 | `sensitive = true` fields redacted in exports/logs | Normal | macros,security | Local file | Inline src security codegen tests (`is_sensitive_effective`) | tests/e2e/macro_e2e.rs |
| MAC-06 | `flatten = true` field merged into the parent namespace | Boundary | macros | Local file | Inline src parse/codegen tests → to add integration | tests/e2e/macro_e2e.rs |
| MAC-07 | `skip = true` field excluded from loading | Boundary | macros | Local file | Inline src codegen tests → to add integration | tests/e2e/macro_e2e.rs |
| MAC-08 | `dynamic = true` field generates a DynamicField handle | Normal | macros,dynamic | None | Inline src codegen tests + examples/dynamic_fields; integration level → to add | tests/e2e/macro_e2e.rs |
| MAC-09 | `merge_strategy` six legal values + illegal value (`"bogus"`) macro error | Normal/Negative | macros | None | Inline src `macros/src/parse.rs` strategy-validation tests; trybuild pinned (merge_strategy_bogus.rs) | tests/e2e/macro_e2e.rs |
| MAC-10 | `env_prefix` struct attribute: prefix stripping + uppercase-underscore mapping | Normal | macros,env | None | `tests/core/derive.rs::test_prefixed_config_env_mapping/test_prefixed_config_with_env_var` | tests/e2e/macro_e2e.rs |
| MAC-11 | `profile`/`profile_env` struct attributes: APP_ENV drives the profile overlay | Boundary | macros | Local file | Inline src parse tests (effective_profile_env); integration level → to add | tests/e2e/macro_e2e.rs |
| MAC-12 | `watch = true` struct attribute: generates watcher wiring (compiles with the watch feature) | Boundary | macros,watch | Local file | Inline src codegen watch tests; integration level → to add | tests/e2e/macro_e2e.rs |
| MAC-13 | `version = N` struct attribute passed through to ConfigMigration (migration version declaration) | Normal | macros,migration | None | `tests/core/derive.rs::test_config_migration_derive_generates_versioned` | tests/e2e/macro_e2e.rs |
| MAC-14 | `ConfigClap` derive: CLI argument parsing and clap App construction (custom long/short via `name_clap_long/short`) | Normal | macros,cli | None | `tests/core/derive.rs::test_config_clap_derive_parses_args/test_config_clap_derive_clap_app`; examples/cli_integration | tests/e2e/macro_e2e.rs |
| MAC-15 | env type-inference edges: bool casing, integer bounds, float notation, network values (1.2.3.4) kept as strings, JSON array strings kept as strings | Boundary | macros,env | None | `tests/core/env_types.rs` (full group of 11 cases) | tests/e2e/macro_e2e.rs |
| MAC-16 | env key parsing does not panic: empty segments `A__B`, unicode, mixed case | Boundary | macros,env | None | `tests/core/env_types.rs::test_env_parse_key_empty_segments_do_not_panic/test_env_parse_key_unicode_and_case` | tests/e2e/macro_e2e.rs |
| MAC-17 | Illegal `encrypt` algorithm / illegal `env_prefix` characters → macro-expansion error (darling span reporting, trybuild acceptance) | Negative | macros | None | Inline src parse validate tests (logic); trybuild pinned (encrypt_rot13.rs, env_prefix_illegal_chars.rs) | tests/e2e/macro_e2e.rs (trybuild) |
| MAC-18 | `interpolate = true` field attribute + interpolation feature: field value interpolated from templates then injected | Boundary | macros,interpolation | Local file | Inline src codegen interpolate tests; integration level → to add | tests/e2e/macro_e2e.rs |

### 2.26 Feature Combination Interactions (CMP, 17 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| CMP-01 | encryption+watch: hot-reloaded file carries new ciphertext; after reload, decryption succeeds with the new key from EnvKeyProvider; with a wrong key → reload fails and rolls back | Normal/Negative | encryption,watch | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-02 | migration+snapshot: load v1 config → migrate to v2 → automatic snapshot → load_snapshot yields v2 content | Boundary | migration,snapshot | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-03 | dynamic+toggle: after the toggle is switched off, dynamic callbacks stop firing; restored when switched on | Boundary | dynamic,feature-toggle | None | None → to add | tests/e2e/combo_e2e.rs |
| CMP-04 | watch+config-bus: FsWatcher detects the file change → publishes ConfigChangeEvent → subscribers perform the reload | Normal | watch,config-bus | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-05 | watch+migration: file reverted to an old version → MigrationOnReload::Always auto-migrates and applies | Boundary | watch,migration | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-06 | progressive-reload+validation: health check includes garde validation; bad config rolls back under Canary | Negative | progressive-reload,validation | Local file | `tests/core/progressive.rs::test_real_health_check_critical_on_invalid` (partial) → file-level end-to-end needed | tests/e2e/combo_e2e.rs |
| CMP-07 | remote+encryption: remote (HTTP polling) delivers encrypted fields → decrypted and injected with the local key | Normal | remote,encryption | Consul(8500) (as HTTP source) | None → to add | tests/e2e/combo_e2e.rs |
| CMP-08 | etcd polling as hot reload: KV change → poll detects → DynamicField updates | Boundary | etcd,dynamic | etcd(2379) | None → to add | tests/e2e/combo_e2e.rs |
| CMP-09 | audit+key rotation: after KeyRing rotate, audit records the rotation event and the old version can still decrypt | Normal | audit,key | Local file | `tests/security/audit.rs::test_audit_log_key_rotation` (one side) → combination needed | tests/e2e/combo_e2e.rs |
| CMP-10 | security-rules+validation two-layer validation: garde structural validation + registry security validation chained, layered report output | Negative | validation,security-rules | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-11 | modules+profile: the ModuleRegistry's active-profile file joins the main ConfigBuilder chain | Boundary | modules,env | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-12 | interpolation+env+file: the file template `${DB_HOST}` is resolved from env, and env is injected by CLI `--env-file` | Normal | interpolation,env | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-13 | cli+schema: `ConfigClap` argument overrides → validate/export commands reflect CLI-priority values | Boundary | cli,macros | Local file | examples/cli_integration (partial) → assertion-level addition needed | tests/e2e/combo_e2e.rs |
| CMP-14 | full-preset all-features smoke test: everything stacked in one pass — build + hot reload + encryption + snapshot + audit all pass | Normal | full | Local file | examples/full_stack (verified by running) | tests/e2e/combo_e2e.rs |
| CMP-15 | snapshot+watch: automatic snapshot before each reload; after 3 consecutive reloads, rollback to any history point | Boundary | snapshot,watch | Local file | None → to add | tests/e2e/combo_e2e.rs |
| CMP-16 | nats-bus dual-instance config sync: instance A hot reloads → NATS broadcast → instance B reloads consistently | Boundary | nats-bus,watch | NATS(4222) | None → to add | tests/e2e/combo_e2e.rs |
| CMP-17 | context-aware+feature-toggle+dynamic: region context + toggle jointly determine the dynamic field value | Boundary | context-aware,feature-toggle,dynamic | None | None → to add | tests/e2e/combo_e2e.rs |

### 2.27 Concurrency and Races (CCY, 8 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| CCY-01 | `InMemoryConfig` concurrent set/get/delete/has across tasks: no deadlock, eventually consistent | Boundary | default | None | Inline src concurrency tests in `src/impl_/memory.rs`, `tests/core/dynamic.rs::test_real_config_extended` | tests/e2e/concurrency_e2e.rs |
| CCY-02 | Concurrent writer appending during watch (each write is full content) → every triggered reload parses completely (no half-read) | Boundary | watch | Local file | None → to add (testified version of bench concurrent_rw_bench) | tests/e2e/concurrency_e2e.rs |
| CCY-03 | Concurrent reload + dynamic read: arc-swap guarantees readers see exactly one of the two versions | Boundary | dynamic,watch | Local file | `tests/core/progressive.rs::test_immediate_reload_atomic` (partial) → stress-level addition needed | tests/e2e/concurrency_e2e.rs |
| CCY-04 | snapshot: concurrent save of many snapshots + simultaneous prune → snapshot count in the directory ≤ max with no corrupted files | Boundary | snapshot | Local file | None → to add | tests/e2e/concurrency_e2e.rs |
| CCY-05 | bus: 4 producers concurrently publish 1000 events, 2 subscribers receive all of them combined (when InMemory capacity suffices) | Boundary | config-bus | None | None → to add | tests/e2e/concurrency_e2e.rs |
| CCY-06 | audit: 8 threads concurrently write events → file parses line-by-line as JSON and the line count is correct | Boundary | audit | Local file | None → to add (stress tier of AUD-11) | tests/e2e/concurrency_e2e.rs |
| CCY-07 | toggle: highly concurrent toggle mixed with is_enabled reads, no panic, final list state self-consistent | Boundary | feature-toggle | None | `tests/core/toggle.rs::test_concurrent_toggle_operations` (basic tier) → stress tier needed | tests/e2e/concurrency_e2e.rs |
| CCY-08 | Very large config (10k keys / depth 32 / 10MB file) loads within limits, rejected beyond them; concurrent reads during load see no tearing | Boundary | default | Local file | Inline src limits tests (rejection side) → large-file integration needed | tests/e2e/concurrency_e2e.rs |

### 2.28 Feature Preset Compile Matrix (PRS, 8 scenarios)

| ID | Scenario Description | Type | Features Involved | Required Service | Existing Coverage | E2E Target |
|----|---------|------|-------------|---------|---------|---------|
| PRS-01 | `--no-default-features` (core only) compiles the whole crate + core unit tests pass | Boundary | — | None | Partially covered by CI → pin as a script | tests/e2e/presets_e2e.rs (compile gate) |
| PRS-02 | `--features minimal` (dotenv+json) compiles | Boundary | minimal | None | None → to add | tests/e2e/presets_e2e.rs |
| PRS-03 | `--features recommended` (toml,dotenv,validation,json,security-rules) compiles | Boundary | recommended | None | None → to add | tests/e2e/presets_e2e.rs |
| PRS-04 | `--features dev` (12 items) compiles + tests/core passes | Boundary | dev | None | None → to add | tests/e2e/presets_e2e.rs |
| PRS-05 | `--features production` (14 items) compiles | Boundary | production | None | None → to add | tests/e2e/presets_e2e.rs |
| PRS-06 | `--features distributed` (8 items) compiles | Boundary | distributed | None | None → to add | tests/e2e/presets_e2e.rs |
| PRS-07 | `--features full` (39 official features; excludes `hot-reload-kit` and pending-removal compatibility aliases) compiles + all `[[test]]` pass (guard-skipped when docker services are missing) | Boundary | full | All | Routine in current CI → pin as script assertions | tests/e2e/presets_e2e.rs |
| PRS-08 | Each single feature enabled one at a time (all 51 `[features]` keys, i.e. 40 official single features + 6 presets + 3 compatibility aliases `env`/`schema`/`key` + the internal aggregate `async-core` + `default`) passes `cargo check` (dependency chains activate automatically: security-rules→security, nats-bus→config-bus etc.; security and encryption are decoupled — security alone does not pull in the encryption stack) | Boundary | All single features | None | None → to add (loop script) | tests/e2e/presets_e2e.rs |

Cross-OS branches of PRS-01 / PRS-07: the CI `build` job uses a 3-OS matrix (ubuntu/macos/windows) to run `cargo build --workspace --all-features` and `--no-default-features` respectively. Its verification boundaries:

- **CI runner exclusive**: the real compilation of `--all-features` on macOS/Windows. That combination pulls in reqwest→rustls→aws-lc-sys and the C/TLS stacks of etcd/nats; cross-compilation requires the target OS's C toolchain and SDK and cannot be reproduced locally; the actual result is produced by the GitHub Actions `build` job.
- **Local pinning (input basis)**: `cargo test --test presets_e2e ci_build_matrix` asserts that ci.yml's matrix contains 3 OSes, `runs-on: ${{ matrix.os }}`, and the two `cargo build` commands above, preventing the matrix from being shrunk or the commands from being changed to a single-feature scope.
- **Local cross-check (equivalent evidence)**: feature sets that do not depend on the C toolchain can be `cargo check`-ed against a target triple; verified locally with exit 0 (requires `rustup target add x86_64-pc-windows-msvc x86_64-apple-darwin` first):

  ```bash
  cargo check -p confers --lib --no-default-features --target x86_64-pc-windows-msvc
  cargo check -p confers --lib --target x86_64-pc-windows-msvc \
    --features "cli,validation,watch,encryption,security,security-rules,key-management,json-schema,typescript-schema,dynamic,progressive-reload,audit,migration,snapshot,interpolation,feature-toggle,lazy-parse,modules,tracing,context-aware,canary,change-stream,config-bus,toml,json,yaml,ini,dotenv"
  # Same as above (macOS): --target x86_64-apple-darwin
  ```

---

## 3. Feature Exclusivity / Combination Matrix

### 3.1 Dependency Chains (fixed by Cargo.toml `[features]`; the parts combination tests need not verify separately)

```text
security          → zeroize, sha2, hex（§9.2-5 解耦：不隐含 encryption/加密栈）
security-rules    → security, ipnet
key-management    → encryption, chrono, rand, hex（key 为兼容别名 → key-management）
cli               → clap, similar, toml, json, yaml, chrono, json-schema
progressive-reload→ watch, arc-swap, async-trait
snapshot          → chrono, tokio, json, toml, yaml, dynamic
etcd              → remote, etcd-client, toml, json, yaml
consul            → remote, toml, json, yaml
nats-bus          → config-bus, async-nats
redis-bus         → config-bus, redis, async-stream
typescript-schema → json-schema（schema 为兼容别名 → json-schema）
dotenv            → dotenvy（env 为兼容别名 → dotenv）
default           → toml, json, dotenv
```

Conclusion: **no mutually exclusive features exist**; all features can be stacked freely; constraints only manifest as "enabling a child automatically brings in its parent". The E2E combination matrix verifies interaction behavior per the explicit combination list in 3.2 (compile compatibility is guaranteed by the PRS matrix).

### 3.2 Combination Test Matrix (input basis for CMP-01…17)

| Combination | Constituent features | Interaction Point | Corresponding Scenario |
|------|-------------|--------|---------|
| C1 | encryption + watch | re-decrypt after reload; key-rotation linkage | CMP-01 |
| C2 | migration + snapshot | snapshot version consistency after migration | CMP-02 |
| C3 | dynamic + feature-toggle | toggle gates callbacks | CMP-03 |
| C4 | watch + config-bus | file events → bus broadcast | CMP-04 |
| C5 | watch + migration | old-version rollback auto-migrates | CMP-05 |
| C6 | progressive-reload + validation | health-check gate | CMP-06 |
| C7 | remote + encryption | remote ciphertext delivery | CMP-07 |
| C8 | etcd + dynamic | remote polling drives dynamic fields | CMP-08 |
| C9 | audit + key | rotation leaves an audit trail | CMP-09 |
| C10 | validation + security-rules | two-layer validation | CMP-10 |
| C11 | modules + env | profile env switching | CMP-11 |
| C12 | interpolation + env | template + env-file injection | CMP-12 |
| C13 | cli + macros(ConfigClap) | CLI override priority | CMP-13 |
| C14 | full bundle | all-features smoke | CMP-14 |
| C15 | snapshot + watch | snapshot before reload | CMP-15 |
| C16 | nats-bus + watch | cross-instance hot-reload sync | CMP-16 |
| C17 | context-aware + feature-toggle + dynamic | three-way joint valuation | CMP-17 |
| C18 | security + remote (VaultKeyProvider) | remote key provider | ENC-17 |
| C19 | snapshot preset hard dependencies (json/toml/yaml/dynamic) | feature chain integrity | PRS-07 |

### 3.3 Preset Coverage Relations (preset → expansion list, verified by PRS-02…07)

| Preset | Expansion | Notes |
|------|---------|------|
| default | toml, json, dotenv | minimal usable set |
| minimal | dotenv, json | no toml |
| recommended | toml, dotenv, validation, json, security-rules | security-rules implicitly brings security (does not imply encryption) |
| dev | toml, json, yaml, dotenv, cli, validation, json-schema, audit, watch, migration, snapshot, dynamic | snapshot implicitly brings json/toml/yaml/dynamic |
| production | toml, dotenv, watch, encryption, validation, audit, json-schema, cli, migration, dynamic, progressive-reload, snapshot, security-rules, feature-toggle | progressive-reload implicitly brings watch |
| distributed | toml, json, dotenv, watch, validation, config-bus, progressive-reload, audit | bus in-process only |
| full | 39 official features (including etcd/consul/nats-bus/redis-bus/context-aware/modules/typescript-schema/key-management/interpolation/canary…; excludes `hot-reload-kit` and the compatibility aliases `env`/`schema`/`key`) | requires docker services for behavior tests |

---

## 4. Docker Service Requirements Summary

Source: `docker-compose.test.yml` (repository root). Start/stop:

```bash
docker compose -f docker-compose.test.yml up -d        # 启动全部
docker compose -f docker-compose.test.yml up -d nats   # 只启动单个服务
docker compose -f docker-compose.test.yml down         # 停止并移除
```

| Service | Image | Host Port | Health Check | Scenario Domains Requiring It | Existing Behavior Without the Service |
|------|------|---------|---------|---------------|------------------|
| NATS | nats:2.10-alpine (`-js -m 8222`) | **4222** (monitoring 8222 not mapped to host) | `wget http://127.0.0.1:8222/healthz` | NAT-01…07, CMP-16; nats group of `tests/remote/bus.rs` | guarded skip via `port_open("127.0.0.1",4222)` |
| Redis | redis:7 | **16379** (maps container 6379) | `redis-cli ping` | RDS-01…07; redis group of `tests/remote/bus.rs` | port-guard skip |
| etcd | quay.io/coreos/etcd:v3.5.16 | **2379** | `etcdctl endpoint health` | ETC-01…04/07/09/10, CMP-08, CMP-07 (HTTP polling host); `tests/remote/etcd.rs`, source group of `tests/remote/remote.rs` | guarded skip via `is_service_available("http://127.0.0.1:2379/health")` |
| Consul | hashicorp/consul:1.19 (dev + ui) | **8500** | `wget http://127.0.0.1:8500/v1/status/leader` | CSL-01…03/05/07/08, REM-10/11, CMP-07; `tests/remote/consul.rs` | guarded skip via `is_service_available` (Consul dev-mode default token is empty/`dev-…`; pin with ACL disabled at E2E time) |
| Nacos | nacos/nacos-server:v2.4.3 (standalone + embedded storage, `NACOS_AUTH_ENABLE=false`) | **8848** (gRPC 9848 mapped likewise) | `curl http://127.0.0.1:8848/nacos/v1/console/health/readiness` (JVM cold start is slow; health check includes a 30s `start_period`) | Nacos config source (`nacos` feature): `tests/remote/nacos.rs` (real publish → poll parse → server-side change → group/namespace scope isolation) | guarded skip via `is_service_available(.../readiness)` |

Service-dependency-free domains (local file / pure in-memory; runnable in CI sandboxes without docker): FMT, BLD, VAL, IPL, ENC, KEY, SEC, AUD, WAT, PGR, DYN, TGL, MIG, SNP, MOD, CTX, SCH, CLI, MAC, CCY, PRS.
**E2E principle**: the integration/E2E layers forbid mocks; scenarios requiring services must hit the real compose services; "remote unreachable"-style negative scenarios always use explicitly unreachable addresses (e.g. `127.0.0.1:1`), never relying on network flakiness.

---

## 5. Execution Plan

### 5.1 Layers and Commands

| Layer | Content | Command | Mocks allowed? |
|------|------|------|-----------|
| L1 unit | 98 `#[cfg(test)]` modules inside src (2433 inline tests) + macros inline tests | `cargo test --features full --lib` | allowed |
| L2 integration | `tests/{core,security,remote,watcher,cli}` (the dead-file issue of §Key Findings-1 is resolved; `error` is registered back in `tests/core/mod.rs`) | `cargo test --features full --test core --test security --test watcher --test cli`; `docker compose -f docker-compose.test.yml up -d && cargo test --features full --test remote` | **mocks forbidden** (the remote group hits real services) |
| L3 examples | run each of the 22 examples (see 5.2) | `cargo run -p confers-examples --bin <name>` | mocks forbidden (service-dependent examples need compose up first) |
| L4 E2E | `tests/e2e/` (the §2 scenario-ID targets of this document) | 33 `[[test]]` sections registered (`autotests=false`); `cargo test --features full --test '*_e2e' --test cli_coverage_extra` (32 targets named like `<domain>_e2e`, e.g. `format_e2e`, `watch_e2e`; `cli_coverage_extra` is an off-matrix regression target that does not follow that suffix) | mocks forbidden |
| Macro tests | `macros/tests` (trybuild compile-failure cases and attribute validation) | `cargo test -p confers-macros` | allowed |
| Fuzzing | `fuzz/` (cargo-fuzz targets: `parser`, `merger`, `interpolation`) | `cargo fuzz run parser` (from the `fuzz/` directory) | mocks forbidden |
| Benchmarks | `benches/` (10 Criterion groups, list in [Performance Guide · Benchmark Suite](PERFORMANCE.md#基准测试套件)) | `cargo bench --features dev,etcd-watch,change-stream --benches` (CI scope; the 3 groups with `required-features=["interpolation"]` additionally need `--features interpolation`) | — |
| Doc tests | public-API rustdoc examples | run with `cargo test --workspace` | — |

Preparatory fixes (complete before landing E2E):
1. ✅ Done: `tests/core/mod.rs` has registered `mod error;` (42 tests revived); the duplicate `tests/core/encryption.rs` has been deleted.
2. ✅ Done: `Cargo.toml` has registered `[[test]]` sections for all 33 `tests/e2e/*.rs` (declaring `required-features` as needed, e.g. `cli_coverage_extra`).

### 5.2 examples Run Checklist (22 total, `cargo run -p confers-examples --bin <name>`, confers dependency pinned to `features=["full"]`)

| # | Example bin | Required Service | Expected Result (acceptance criteria) |
|---|---------|---------|----------------------|
| 1 | `basic_usage` | None | exit code 0; prints the loaded config values (defaults + env override demo) |
| 2 | `hot_reload` | Local file | exit code 0; demonstrates new values taking effect after file modification (includes a self-edit self-read script segment) |
| 3 | `remote_consul` | Consul(8500) | exit code 0; three demos (basic poll / token / builder limits) all print their completion banners |
| 4 | `remote_etcd` | etcd(2379) | exit code 0; without the service, prints an etcd startup hint and exits gracefully (built-in fallback path) |
| 5 | `encryption` | None | exit code 0; encrypt/decrypt roundtrip and redaction demo output |
| 6 | `key_rotation` | None | exit code 0; KeyManager/rotation-plan demo output |
| 7 | `migration` | Local file | exit code 0; v1→v2 migration demo output |
| 8 | `dynamic_fields` | None | exit code 0; field updates firing callbacks output |
| 9 | `config_groups` | Local file | exit code 0; group/profile loading output |
| 10 | `progressive_reload` | Local file | exit code 0; canary/rollback demo output |
| 11 | `full_stack` | None | exit code 0; all-features (Config+dynamic+secret+watcher) demo output |
| 12 | `config_bus` | None | exit code 0; InMemoryBus publish / multi-subscriber receive output |
| 13 | `snapshot` | Local file | exit code 0; snapshot save/list/restore output |
| 14 | `validation` | None | exit code 0; both branches output — valid passes + invalid rejected |
| 15 | `cli_integration` | Local file | exit code 0; ConfigClap argument-parsing demo output |
| 16 | `json_schema` | None | exit code 0; JSON Schema and TS type generation output |
| 16b | `schema_to_rust` | None | exit code 0; JSON Schema draft → Rust struct scaffolding (basic mapping / $defs / $ref / fail-loud segments) output |
| 17 | `interpolation` | Local file/None | exit code 0; `${VAR}` and `${VAR:default}` interpolation output |
| 18 | `audit` | Local file | exit code 0; audit events written to a temp directory output |
| 19 | `context_aware` | None | exit code 0; context-switch valuation output |
| 20 | `modules_demo` | Local file (some segments mention consul/etcd strings for display only) | exit code 0; module registration and active-profile loading output |
| 21 | `security` | None | exit code 0; security validation / redaction demo output |

Batch command: `set -e; for b in $(cargo metadata --format-version 1 | jq -r '.packages[]|select(.name=="confers-examples")|.targets[].name'); do cargo run -q -p confers-examples --bin "$b"; done`

### 5.3 Scenario ID → E2E File Mapping Summary

| tests/e2e/ File | Covered Scenarios | Needs docker |
|----------------|---------|-----------|
| format_e2e.rs | FMT-01…22 | no |
| builder_e2e.rs | BLD-01…28 | no |
| validation_e2e.rs | VAL-01…08 | no |
| interpolation_e2e.rs | IPL-01…08 | no |
| encryption_e2e.rs | ENC-01…20, ENC-22 | no |
| key_e2e.rs | KEY-01…13 | no |
| security_rules_e2e.rs | SEC-01…18 | no |
| audit_e2e.rs | AUD-01…11 | no |
| watch_e2e.rs | WAT-01…18 | no |
| progressive_e2e.rs | PGR-01…12 | no |
| dynamic_e2e.rs | DYN-01…14 | no |
| toggle_e2e.rs | TGL-01…05 | no |
| migration_e2e.rs | MIG-01…10 | no |
| snapshot_e2e.rs | SNP-01…10 | no |
| modules_e2e.rs | MOD-01…11 | no |
| context_e2e.rs | CTX-01…08 | no |
| remote_e2e.rs | REM-01…11, ETC-01…10, CSL-01…08 | Consul/etcd (REM-10/11, CSL-07/08, ETC-09/10, CMP-07/08) |
| bus_e2e.rs | BUS-01…10, NAT-01…06, RDS-01…07 | NATS/Redis (NAT/RDS groups) |
| schema_e2e.rs | SCH-01…04 | no |
| cli_e2e.rs | CLI-01…24, SCH-05 | no |
| cli_coverage_extra.rs | Off-matrix regression: CLI coverage supplements (snapshot restore/prune/diff, doctor health branches, docs knowledge pack, schema --from-instance, get redaction/`--env-file`, export all formats and directory output; required-features = cli,snapshot,encryption,yaml) | no |
| macro_e2e.rs | MAC-01…18 (including trybuild: ENC-21, ENC-23, MAC-17) | no |
| combo_e2e.rs | CMP-01…17, TGL-06, CTX-09, IPL-09 | CMP-07/08/16 need Consul/etcd/NATS |
| concurrency_e2e.rs | CCY-01…08 | no |
| presets_e2e.rs | PRS-01…08, SCH-03 | no |
| default_attrs_e2e.rs | Off-matrix regression: bare-word form of `#[config(default)]` and the `default = None` edge | no |
| probe_sensitive.rs | Off-matrix regression: sensitive-field `<VAR>_FILE` references (valid path read, missing path hard error, fallback to the plain variable without `_FILE`, t012 series) | no |

### 5.4 Suggested Execution Order

1. ✅ Dead-file fix complete (§5.3 preparatory fix 1) → 2. L1+L2 all green (without docker, run the non-remote groups first) → 3. start the 5 compose services and run L2 remote + the 22 L3 examples → 4. land scenarios file by file in e2e (prioritize CMP/CCY/negative classes, since their existing coverage is thinnest) → 5. PRS compile gates into CI (one round per PR and per nightly).

---

## 6. Statistics Summary

(Computed programmatically row by row over the scenario rows of this document; the counting scope is in the notes)

| Dimension | Count |
|------|------|
| Total scenarios | **357** |
| Type=Normal (pure) | 152 |
| Type=Negative (pure) | 84 |
| Type=Boundary (pure) | 110 |
| Type=Normal/Negative (dual assertion) | 8 |
| Type=Normal/Boundary (dual assertion) | 3 |
| E2E scenarios needing additions (existing coverage "None → to add") | 44 |
| Scenarios needing added assertions/integration (unit tests or examples exist, but integration/behavior-level assertions are missing, "…→ to add") | 36 |
| Scenarios referencing existing `tests/` integration tests (overlaps other scopes) | 222 |
| Scenarios referencing inline src tests (overlaps other scopes) | 91 |
| Scenarios referencing `⚠dead file` tests (originally 18 places; §Key Findings-1 resolved, markers fully removed) | 0 |
| Scenarios verified via examples | 9 |
| Scenarios depending on docker services (any of NATS/Redis/etcd/Consul) | 30 |
| Scenarios depending only on local files | 139 |
| Scenarios with no external dependency at all (pure in-memory / compile-time) | 188 |

> Note: existing-coverage scopes legitimately overlap (one scenario may reference both tests/ integration tests and inline src tests); "to add" takes the strongest-gap verdict. The follow-up implementation phase may merge overly fine-grained scenarios, but **IDs stay stable** for traceability.

### In-Code Test Scale

(Counted by grepping `#[test]` / `#[tokio::test]` attribute lines, as of the v0.6.0 workspace; `tests/` sums to 785 lines, 1 of which is a mention inside a doc comment of `tests/remote/etcd_watch.rs`; anchored to line-start attributes, 784 test functions)

| Category | Count |
|------|------|
| Unit tests (inline `#[cfg(test)]` modules in `src/`, 98 modules) | 2433 |
| Integration and E2E (`tests/`, 67 files, including the shared `common.rs` pulled in by four mod.rs) | 784 |
| E2E suites (`tests/e2e/`, explicitly registered via `[[test]]`) | 33 |
| Fuzz targets (`fuzz/`) | 3 |
| Criterion benchmark groups (`benches/`) | 10 |

The coverage gate requires line coverage of no less than 90%, enforced doubly by CI and the pre-push hook.

---

## 📚 Related Documents

| Document | Description |
|:-----|:-----|
| [🏗️ Architecture](ARCHITECTURE.md) | modules and data flows involved in the scenarios |
| [📘 API Reference](API_REFERENCE.md) | public APIs referenced by the scenarios |
| [⚡ Performance Guide](PERFORMANCE.md) | benchmark suite and performance baselines |
| [🤝 Contributing](CONTRIBUTING.md) | test-writing and commit conventions |
