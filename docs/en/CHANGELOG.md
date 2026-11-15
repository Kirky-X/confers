# Changelog

This file records all significant changes to this project.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2026-11-14

The official release following rc.13 (2026-11-04): no API changes; rc.13 users only need to bump the version number to upgrade.

### Changed

- **CLI binary entry migration**: the custom `[[bin]]` path (`src/cli/main.rs`) violated the unified binary convention; migrated to a thin shell at the root `src/main.rs` (calling `confers::cli::run`, with `required-features = ["cli"]`, exit-code contract, and i18n initialization behavior unchanged); `src/cli/` retains only the library module. The CLI framework remains clap as-is: sdforge depends on confers, and replacing it would create a reverse dependency cycle — a structural exemption
- **i18n foundation exception registered**: under the dependency-cycle constraint, the self-hosted FTL engine is registered as a foundation exception, not a family-spec violation (recorded in comments in `src/i18n/mod.rs`)

### Dependency Upgrades

- `cargo update` refreshed 28 candidate packages, 27 landed (all semver-compatible position bumps; no cross-position version jumps in the runtime dependency graph): the prost four-piece set 0.14.4→0.14.6, notify 8.2.1→8.2.2, notify-types 2.2.0→2.3.0, toml 1.1.8→1.2.1 (with toml_datetime 1.1.2→1.1.3, toml_writer 1.1.3→1.2.0, serde_spanned 1.1.2→1.1.3, winnow 1.0.4→1.0.5), syn 3.0.9→3.0.10, serde-saphyr 2.1.0→2.1.1, uuid 1.29.0→1.29.2, arc-swap 1.9.3→1.9.5, cc 1.11.0→1.11.2, mio 1.2.4→1.2.5, rustls-pki-types 1.15.1→1.16.0, rustls-webpki 0.103.16→0.103.17, webpki-root-certs 1.0.11→1.0.12, fixedbitset 0.5.7→0.5.9, smallvec 1.16.4→1.16.5, itoa 1.0.18→1.0.19, annotate-snippets 0.12.17→0.12.18, errno 0.3.14→0.3.15, inout 0.2.5→0.2.6, mime 0.3.17→0.3.18
- itertools pinned back precisely 0.15.0→0.13.0: prost-build 0.14.6's requirement allows 0.13; after the rollback it unifies with the version criterion uses, eliminating a 0.x cross-position duplicate
- prettyplease 0.3.0 newly entered the compile-time codegen chain (prost-build 0.14.6 mandates `^0.3`, coexisting with tonic-build's 0.2.37; both are build-dependencies and stay out of the runtime dependency graph; the rollback attempt was rejected by constraints and accepted with all gates green)
- Held in place by constraints: async-nats 0.51.0 (0.52 would require raising the Cargo.toml version requirement — a future proactive upgrade), matchit 0.8.4 (held by tonic's version constraint)
- The fuzz directory (workspace-excluded) Cargo.lock only synced the confers/confers-macros version 0.6.0-rc.13→0.6.0; no third-party dependency changes

### CI

- dependabot actions upgrades (SHA pins): codecov/codecov-action re-pinned to the official v7.1.1 (#66), Swatinem/rust-cache v2.9.3 (#67), arduino/setup-protoc v3.0.0 (#68), github/codeql-action init/analyze v4.39.2 (#69, #70)

## [0.6.0-rc.13] - 2026-11-04

### Fixed

- **File-watch establishment race (R-watch-008)**: `FsWatcher::new()` / `MultiFsWatcher::new()` previously returned immediately while the OS-level watch was established later by a background thread — a caller writing to the file right after `new()` returned would **permanently lose** that event (macOS FSEvents does not replay changes from before a stream starts; reproducible in the single-watcher sequential case, worse under concurrency; the concurrent failures in fluxgate's hot-update tests were previously misattributed to a "kqueue platform defect" when they were a defect of this library). Now watch establishment completes synchronously before the constructor returns: an `Ok` return means the watch is live, and any modification thereafter is guaranteed deliverable; establishment failure now makes the constructor return `Err` directly (`ConfigError::WatcherError`) instead of producing a watcher that "looks alive but is deaf". Post-establishment failure semantics are unchanged (`is_running()` flips to `false`, `recv()` returns `None`). Regression tests: `tests/watcher/establishment.rs` (single watcher immediate write + 9 watchers concurrent immediate writes)
- **Test diagnostics no longer print derived values**: the panic message for master-key verification failure in `tests/e2e/gap_sec_e2e.rs` dropped the `api_key` field-length output (CodeQL `rust/cleartext-logging` judges the length as sensitive-value-derived info usable as a side channel). Assertion semantics unchanged: the `Ok` branch still panics and the `Err` branch still checks that the error text contains decrypt/encrypt/key
- **Watching a directory tree with unreadable subdirectories now fails explicitly**: with watch establishment synchronized, recursively watching a tree containing an unreadable subdirectory (e.g. `systemd-private-*` under `/tmp` on Linux) returns `Err(ConfigError::WatcherError)` at construction instead of the previous silent success. This is the inevitable consequence of "never producing a watcher that looks alive but is deaf"; affected usages should point at readable concrete subdirectories instead. The same assertion on the CI side now uses a dedicated temp directory, avoiding dependence on the host's `/tmp` readability

### Dependency Upgrades and Feature Narrowing (third-party version bumps + feature-narrowing consolidation)

- Direct dependency upgrades: `serde-saphyr` 1.3→2.1 (across a major version): compared item by item against the downloaded 2.1.0 sources; the signatures of this repo's usage points (`Error`, `from_str`, `from_str_with_options`, the `strict_booleans` field of the `options!` macro, `to_string`) are unchanged; the `serialize`/`deserialize` features are retained, and the upstream default feature `comments` is excluded via `default-features = false` (YAML parsing semantics unchanged); MSRV 1.89 is below this repo's 1.97.1
- Direct dependency upgrade: `async-nats` 0.50→0.51 (0.x minor, a semver-breaking position): diffed the 0.50/0.51 sources; all of this repo's usage points (`get_or_create_stream`, `create_consumer`, the pull `Config` fields `deliver_policy`/`durable_name`/`name`/`inactive_threshold`/`max_deliver`, `messages()`, `AckKind::Nak(Some(delay))`, `ConnectOptions`) are preserved; all changes are additions (priority groups / pinned client, gated by the `server_2_11+` feature only); MSRV 1.89
- Direct dependency upgrades: `tokio` 1.53→1.54, `redis` 1.7→1.8 (stable minors; the `tokio-comp` feature expansion unchanged; `src/bus/redis.rs` only uses `Client`/`AsyncCommands`/`aio::PubSub`)
- Feature narrowing (`default-features = false` unchanged, root and `[workspace.dependencies]` in sync): `garde` drops `url`/`regex` (a repo-wide grep shows only the `#[garde(length/range/email)]` attributes in use; the `url` and `pattern` attributes have zero usage points; rc.10 previously kept them for the CONFIG_MACRO_GUIDE doc example, which has now been annotated with the features it needs); all other direct dependencies verified item by item that features map to real API usage — no feature additions needed
- Cargo.lock refresh (`cargo update`, 27 packages, including the direct dependencies above): granit-parser 1.3.0→2.1.0, tonic 0.14.6→0.14.9 (with tonic-build/tonic-prost/tonic-prost-build), aws-lc-rs 1.18.2→1.19.0, aws-lc-sys 0.46.0→0.47.0, async-lock 3.4.2→3.5.0, cc 1.10.0→1.11.0, encoding_rs 0.8.42→0.8.44, notify-types 2.1.0→2.2.0, log 0.4.34→0.4.35, rustix 1.1.5→1.1.6, ctutils 0.4.5→0.4.8, openssl-probe 0.2.1→0.2.2, unicode-width 0.2.2→0.2.3, utf8_iter 1.0.4→1.1.0, the 5 wasm-bindgen family packages (wasm-bindgen/wasm-bindgen-macro/wasm-bindgen-macro-support/wasm-bindgen-shared 0.2.129→0.2.130, wasm-bindgen-futures 0.4.79→0.4.80), web-sys/js-sys 0.3.106→0.3.107, plus the new utf_types 1.0.0 (entering the graph with utf8_iter 1.0.4→1.1.0's new dependency); matchit 0.8.4 held unchanged by tonic's version constraint
- fuzz directory (workspace-excluded) Cargo.lock refresh: the confers path resolution aligned 0.6.0-rc.6→0.6.0-rc.13 with the current workspace; `libfuzzer-sys` 0.4.13 is already the latest stable and untouched
- CI actions upgrades (SHA pins): Swatinem/rust-cache v2.9.3, taiki-e/install-action v2.90.0, arduino/setup-protoc v3.0.0, github/codeql-action v4.39.2, softprops/action-gh-release v3.0.3, rust-lang/crates-io-auth-action v1.0.6, codecov/codecov-action re-pinned to the current target of the official v7.1.1 tag (the previous SHA did not match the tag); actions/checkout v7.0.1 and actions/upload-artifact v7.0.2 verified as already latest and untouched; dtolnay/rust-toolchain remains a toolchain-branch pin (consistent with rust-toolchain.toml 1.97.1)
- Compatibility verification (measured this round): `cargo check --workspace --all-features`, `--no-default-features`, the preset combinations minimal/recommended/dev/production/distributed, and the single features yaml/nats-bus/redis-bus/validation/etcd-watch all finish with 0 errors; `cargo clippy --workspace --all-features -- -D warnings` 0 warnings; targeted tests all green — `format_e2e` 8 passed (the serde-saphyr 2.1 path), `error_scenarios_e2e` 24 passed (including the `#[garde(email)]` rule), `validation_e2e` 5 passed, lib unit tests (yaml feature) 802 passed, lib unit tests (bus feature) 832 passed, `cli` 32 passed, `combo_e2e` (full) 16 passed; the NATS/Redis service-dependent cases self-skip locally (services not running, an explicit skip by design), and the real network paths are covered by the CI test job
- Security audit: `cargo audit` (advisory DB 1403 entries / 388 dependencies) 0 vulnerabilities 0 warnings, `cargo deny check` advisories/bans/licenses/sources all ok, no fix items
- In-house dependency versions untouched: `confers-macros` is a path dependency (workspace member) with version 0.6.0-rc.13 consistent with the workspace; this repo has no optional in-house dependency held at a pinned version

## [0.6.0-rc.12] - 2026-10-25
### Changed (breaking-surface declaration)

- **Deprecated security types removed entirely**: `security::SecureLogger` / `SafeResult` / `SensitiveDataFilter` / `FilterResult` / `LogLevel` (previously only marked `#[deprecated]`) were deleted along with the bare-name `security::ErrorSanitizer` export; the message sanitizer is exposed under the single name `security::SecurityErrorSanitizer`, and the replacement is `confers::error::sanitize::sanitize_error_message`. Downstream code still referencing the old types will fail to compile — an intended break
- **`_FILE` credential path policy tightened** (`PathValidator::validate_and_resolve`, effective via `_FILE` reads in `EnvSource` and the derive macro): **extension-less dotfiles** (e.g. `~/.myapp/.secret`) and paths **hitting a credential directory component** (e.g. `/opt/docker/conf/app.toml`, component `docker`) are rejected outright. The fail-safe direction is right, but this will break existing deployments — usages pointing `_FILE` at dotfiles or docker config directories must rename the file or change the directory first; before upgrading, check each `_FILE` variable's target filename and path components one by one
- **Credential-directory rejection message now names the matched component**: when `PathValidator::validate_and_resolve` hits a credential directory component, the error text changed from a fixed directory list to interpolating the matched component (English: `access to credential directory docker is not allowed`) — the list previously existed as hardcoded copies in the two-language messages and would inevitably miss newly added components (the un-dotted `docker` was rejected yet absent from the message); logs and assertions matching the old text literally need updating in sync

### Security

- **Master-key redaction**: `ZeroizingBytes` now uses a hand-written `Debug` (always `[REDACTED]`) and no longer outputs raw key bytes via `#[derive(Debug)]`; aligned with the existing convention of `SecretBytes`/`SecretString`/`zeroize::Zeroizing`
- **Server-side paths no longer leak**: `SourceLocation` now uses a hand-written `Debug` and no longer prints `pub(crate) file_path` — the field's comment always declared "never exposed to end users", but the `Debug` channel (logs, `unwrap()` panic text, wrapper structs with derived `Debug`) bypassed that invariant
- **Parse errors no longer carry the source text**: on TOML parse failure the whole document held by `toml::de::Error` is cleared (`Debug` and `Display` no longer echo the offending line), and the `source` of `ConfigError::ParseError` keeps only the message and span
- **`_FILE` read TOCTOU**: `EnvSource` now opens the **canonical path** returned by `PathValidator`, closing the symlink-swap window between "validate canonical, open raw" (aligning with the existing correct practice in `impl_/loader.rs` and `macros/src/codegen/load.rs`)
- **`_FILE` read size cap**: added `PathValidator::MAX_SECRET_FILE_SIZE` (10 MiB) and `PathValidator::read_secret_file`; `_FILE` reads in `EnvSource` and the derive macro are uniformly rate-limited through it (previously uncapped — environment variables could trigger unbounded memory allocation)
- **`_FILE` credential-file policy completed**: `PathValidator` rejects sensitive system locations (added `/etc/gshadow`, `/etc/sudoers`, `/etc/ssh`, etc.), credential directories (added `.docker`), and credential file basenames (`.netrc`, `.npmrc`, `.pypirc`, `.git-credentials`, `credentials`, etc.) — previously these extension-less files completely skipped the extension whitelist because `Path::extension()` returned `None`
- **Redirects no longer leak credentials**: headers configured via `HttpPolledSourceBuilder::with_header` are sent only on redirects to the **same origin** (scheme + host + port); cross-origin hops no longer carry Bearer credentials (aligning with reqwest's own `remove_sensitive_headers` semantics)
- **Snapshot redaction fallback**: `to_json_with_mode(Redacted, …)` gained a key-name fallback (`sensitive_names::is_sensitive_name`); sensitive fields not registered in `sensitive_paths` are no longer written to snapshots in plaintext; the derive macro's `sensitive_paths()` now uses `is_sensitive_effective()`, and `SecretString` fields register automatically
- **Export file permissions**: `confers export -o <file>` now uses owner-only (0600) atomic writes, consistent with the disk-write discipline of snapshots/key stores (previously written per the process umask, typically 0644)
- **Conflict report redaction**: `AnnotatedValue::conflict_report` redacts overridden values by key name (aligned with `MergeEngine::report_conflict`'s existing redaction convention), and `AnnotatedValue::conflict_report_with_sensitive_paths` was added for explicitly passing a sensitive-path list
- **Remote path-segment validation**: `VaultKeyProvider`'s `secret_path` and `K8sApiSourceBuilder`'s `namespace`/`name` enforce a path-segment whitelist (rejecting `..` and illegal identifiers), preventing `url` normalization from rewriting requests to another tenant/namespace
- **Full-store reads must be declared explicitly**: an empty prefix in the Consul/etcd sources errors at the `build()` stage; `allow_full_store_read(true)` is required to permit it
- **Error-type stabilization**: `RemoteUnavailable.error_type` no longer writes implementation-detail strings such as the `reqwest::Error` type name, and no longer carries the raw remote response body
- **The second TOML parse path also stops carrying the source text**: the parse path of `confers::format::converter_for(Format::Toml)` (public API), which goes through `impl_/format`, previously did not clear the whole document held by `toml::de::Error`, so the `Debug` of `ConfigError::ParseError` would echo the offending line — now consistent with `impl_/loader.rs`'s handling
- **Config-bus error types stabilized**: `RedisConfigBus` / `NatsConfigBus` no longer concatenate raw driver errors and endpoint addresses into `RemoteUnavailable.error_type`, using stable labels (`redis_connection_failed`, `nats_connect_failed`, etc.) + redacted detail instead
- **URLs with credentials on non-HTTP endpoints now included in redaction**: the credential-URL rule of `sanitize_error_message` previously covered only `http`/`https`; endpoint URLs in `redis://`, `nats://` and other driver errors were not masked — now all schemes are covered
- **Conflict-report redaction scope unified**: `MergeEngine::report_conflict` gained the `sensitive_names::is_sensitive_name` key-name fallback, consistent with `AnnotatedValue::conflict_report` — credential keys not registered in `sensitive_paths` no longer enter conflict reports in plaintext
- **Temp-file permissions on disk writes**: `write_owner_only` (snapshots and `confers export`) now creates temp files exclusively; pre-existing `.tmp` sibling files (crash residue or planted in a shared directory) no longer receive plaintext under their own permissions, and planted symlinks are not written either
- **`_FILE` read symlink fallback**: `PathValidator::read_secret_file` re-verifies after opening that the path still points to the same file (unix compares device/inode, windows compares creation time/modification time/size), rejecting reads when the final segment has been swapped for a symlink

### Added

- `confers::error::sanitize::sanitize_error_message` became a public API (previously `pub(crate)`, while the `security::ErrorSanitizer` docs recommended that external users switch to it)
- `PathValidator::validate_and_read_secret_file`: a one-step validate + read `_FILE` entry point (avoiding callers mistakenly handing unvalidated paths straight to `read_secret_file`)
- `confers::security::SecurityErrorSanitizer`: a disambiguating alias of `security::ErrorSanitizer` (same name as `key::ErrorSanitizer` but different semantics)

### Changed

- The rustdoc of `security::ErrorSanitizer` now explicitly documents the "keep the first 4-8 characters" semantics and points to the reachable full-replacement alternative; `docs/SECURITY.md` and `docs/API_REFERENCE.md` gained the same warning in sync
- `security::SecureLogger` / `SafeResult` / `SensitiveDataFilter` / `FilterResult` / `LogLevel` marked `#[deprecated]`: previously only `security/error_sanitization.rs` itself used these five types; the replacement is `confers::error::sanitize::sanitize_error_message`; **not yet deleted** — removal after confirmation in the next version
- The snapshot-redaction key-name fallback replaces an entire sensitive section with the string `"[REDACTED]"` (not an object) — downstream consumers parsing snapshots structurally must tolerate the node-type mismatch (the semantics are documented in rustdoc)

---

## [0.6.0-rc.11] - 2026-10-19

### Changed

- **YAML backend migration**: `serde_yaml_ng` → `serde-saphyr` (pure Rust, eliminating `RUSTSEC-2024-0449` (the unsafe-libyaml archival advisory) at the root) — parsing unified through the `serde_json::Value` intermediate state (`preserve_order` keeps source order), numeric keys changed from silent filtering to stringified retention, and tag-forced-conversion semantics preserved
- **YAML parsing fix**: generic value parsing unified on `strict_booleans` — saphyr by default infers YAML 1.1 shorthands like `n`/`no`/`off` as booleans, breaking three-format semantic equivalence (`name: n` is a string in TOML/JSON); typed targets keep the default lenient behavior and are unaffected
- **License**: unified to pure MIT; the Commons Clause section was removed from LICENSE and the manifest declares `license = "MIT"`

### Fixed

- **Tests**: after `preserve_order`, schema/cli key-order assertions aligned to source order — the `required` list and `rename` field pairings updated with source order, retiring the BTreeMap-sorting-era expectations

### Internal

- **CI releases**: publish credentials switched to crates.io trusted publishing (OIDC auth-action); added the rc-prerelease derivation and publish-idempotency triple (tag-level concurrency group + post-publish release-uniqueness assertion)
- **Security/repo hygiene**: added gitleaks noise-reduction config and false-positive fingerprints; gitignore now ignores design documents; README and docs anchor-link fixes

---

## [0.6.0-rc.10] - 2026-10-10

### Dependency Upgrades and Feature Narrowing (third-party version bumps + feature-narrowing consolidation)

- Direct dependency upgrade: `etcd-client` 0.20→0.21 (0.x minor, a semver-breaking position): the upstream 0.21.0 release notes are purely additive (etcd 3.7 streaming range reads, `MaintenanceClient::downgrade`, etcd 3.6 response fields); the `tls` feature expansion (`tls` → `tls-ring`) matches 0.20 item by item, and this repo's usage points (`Client`/`ConnectOptions`/`TlsOptions`/`GetOptions`/`EventType`/`WatchOptions`) have no signature changes
- Compatibility verification method (measured this round): `cargo check --workspace --all-features --all-targets`, `cargo hack check --each-feature` (53/53 combinations, 0 errors), `cargo test --all-features --lib` (2404 passed / 0 failed), `cargo test --all-features --test remote` (74 passed, covering the cases of `src/remote/etcd.rs` and `src/remote/etcd_watch.rs`)
- The previous checkpoint's transitive patch refresh (Cargo.lock, 21 packages, dependency version requirements untouched): h2 0.4.19→0.4.20, hyper 1.11.1→1.12.0, tokio 1.53.1→1.53.2, libc 0.2.189→0.2.190, mio 1.2.3→1.2.4, cc 1.5.1→1.6.0, trybuild 1.0.121→1.0.122, unicase 2.9.0→2.10.0, want 0.3.1→0.3.2, zerocopy 0.8.59→0.8.62, zerocopy-derive 0.8.59→0.8.62, zeroize 1.9.0→1.9.1, uuid 1.26.1→1.27.0, toml 1.1.6→1.1.7, toml_datetime 1.1.1→1.1.2, toml_parser 1.1.3→1.1.4, toml_writer 1.1.2→1.1.3, serde_spanned 1.1.1→1.1.2, powerfmt 0.2.0→0.2.1, either 1.18.0→1.19.0, ctutils 0.4.2→0.4.3
- Transitive patch refresh (`cargo update`, Cargo.lock): toml 1.1.7→1.1.8, syn 3.0.6→3.0.7, cc 1.6.0→1.7.0, smallvec 1.16.2→1.16.3, tokio-util 0.7.19→0.7.20, toml_parser 1.1.4→1.1.5, uuid 1.27.0→1.28.0; the `cargo outdated` re-check output "All dependencies are up to date" (matchit 0.8.4 is exactly locked by axum 0.8.9's `=0.8.4`, outside this repo's control)
- Feature narrowing (`default-features = false` unchanged, explicit features minimized, root and `[workspace.dependencies]` in sync to prevent re-flowing back through inheritance): `async-nats` drops `server_2_10`/`server_2_11`/`server_2_12`/`server_2_14` (cfg surface gating newer server APIs only; this repo uses only the core JetStream API, and the real NATS 2.10 JetStream case was re-verified); `clap` drops `env` (no `env` parameter-attribute usage points in the repo); `futures-util` drops `async-await`/`async-await-macro` and the workspace `futures` drops `async-await` (no futures macro usage points); `tracing` drops `attributes` (root + examples; no `#[instrument]` repo-wide); `confers-macros` (in-house, declaring no features) adds `default-features = false` to match the repo-wide per-dependency explicit-declaration convention
- Explicit features retained (with usage points or documented promises, not trimmed): `garde` `email`/`url`/`regex` (the url and `pattern` documentation promises live in the docs/CONFIG_MACRO_GUIDE.md examples), `clap` `color`/`suggestions`/`usage`/`error-context`, `moka` `future`+`sync`, `etcd-client` `tls`
- This round's dependency consolidation (third-party, measured 2026-10-11): deleted the zero-usage dev-dependency `proptest` (`grep -rn proptest --include=*.rs .` hits 0 repo-wide, `git log -S proptest -- src tests benches examples` shows no historical usage; the locked `proptest`/`rand_xorshift`/`core_detect`/`unarray` consequently left the dependency graph); `zeroize` drops `derive` (no `#[derive(Zeroize)]`/`#[derive(ZeroizeOnDrop)]` usage points; `secure_string.rs` is a hand-written `impl ZeroizeOnDrop`; `zeroize_derive` 1.5.0 left the dependency graph); the root `clap` drops `env` to align with `[workspace.dependencies]` (`cargo tree -e features -i clap` output no longer contains `clap feature "env"`); `redis` drops the redundant `aio` (already implied transitively by `tokio-comp`; the feature tree verified unchanged); `confers-macros` (in-house) landed `default-features = false`
- Transitive patch refresh (`cargo update`, Cargo.lock): serde_json 1.0.151→1.0.152; the lock no longer contains the `proptest`/`zeroize_derive`/`rand 0.9` chains
- CI actions upgrades (SHA pins): taiki-e/install-action v2.87.26→v2.87.27, dtolnay/rust-toolchain SHA advanced to master HEAD; checkout/rust-cache/setup-protoc/codecov/upload-artifact/codeql/gh-release verified one by one via gh api as already latest and untouched; on 2026-10-11 a full re-check with `git ls-remote` confirmed all 8 pins (checkout v7.0.1, codeql v4.38.3, upload-artifact v7.0.2, codecov v7.1.1, rust-cache v2.9.2, install-action v2.87.27, setup-protoc v3.0.0, gh-release v3.0.3) equal the upstream latest tags — untouched
- Security audit: `cargo audit` no vulnerabilities (1296 advisories / 377 dependencies), `cargo deny check` advisories/bans/licenses/sources all ok; warn-level notices are duplicate transitive versions, unmatched license whitelist entries, and intentional path-only dev-dependencies — no fix items
- In-house dependency versions untouched: `confers-macros` and the confers references in examples/fuzz remain at 0.6.0-rc.6 (bumped uniformly at the release chain's "in-house dependency pinning" stage)

### 2026-10-08 batch (after rc.6, not tagged)

#### Fixed

- **async-core aggregate feature self-consistency**: added the missing `dep:async-trait` declaration so `async-core` alone compiles; added a feature-matrix workflow covering the feature combination matrix.
- **Default feature combination test gating**: gated the `snapshot_extra` test module per the default feature combination, fixing compile/test failures under the default combination.

#### Removed

- **Dead code and test-only function cleanup**: deleted compiler-confirmed dead code with cfg feature isolation as needed; removed test-only functions from production code; converged test-stub `allow(dead_code)` into precise, reasoned annotations.

#### Changed

- **Complex hotspot convergence**: extracted helper functions from duplicated branches.
- **Governance and docs**: moved the specmark directory and the ws-R14 governance review records out of git tracking; unified the vulnerability reporting channel to GH security advisories first with email as fallback; fixed the API reference key-management anchor link.

## [0.6.0-rc.7] — 2026-10-08

### Fixed

- **async-core aggregate feature self-consistency**: added the missing `dep:async-trait` declaration so `async-core` alone compiles; added a feature-matrix workflow covering the feature combination matrix.
- **Default feature combination test gating**: gated the `snapshot_extra` test module per the default feature combination, fixing compile/test failures under the default combination.

### Removed

- **Dead code and test-only function cleanup**: deleted compiler-confirmed dead code with cfg feature isolation as needed; removed test-only functions from production code; converged test-stub `allow(dead_code)` into precise, reasoned annotations.

### Changed

- **Complex hotspot convergence**: extracted helper functions from duplicated branches.
- **Governance and docs**: moved the specmark directory and the ws-R14 governance review records out of git tracking; unified the vulnerability reporting channel to GH security advisories first with email as fallback; fixed the API reference key-management anchor link.

## [0.6.0-rc.6] — 2026-10-02

### fix-audit-defects-r1 (2026-09-23 audit defect fixes, 52 items)

#### Added

- `#[config(profile)]` landed: when `RUN_ENV` (or the variable named by `profile_env`) is set, the `<stem>.<env>.<ext>` environment-specific file is loaded automatically as an overlay
- The `encrypt` field attribute made real: the load pipeline auto-decrypts unified envelope ciphertext and injects it; `ConfigBuilder::master_key` supports explicit master-key injection (overriding `CONFERS_MASTER_KEY`); Vault login proactively refreshes 10% ahead of `lease_duration` (the 403 reactive re-login remains as a fallback)
- Unified envelope `enc:v1:<keyver>:<payload>` (old format readable for compatibility); the audit chain supports an external HMAC key (`AuditConfig::hmac_key`)
- The `HttpPolledSource::stale_on_error` option (off by default, keeping fail-loud), auth-header injection API, default timeout
- CLI `inspect`/`get` redact by default + `--reveal` for explicit plaintext (stderr warning)
- Public `PathValidator` (crate root, not feature-gated) and the `sensitive_names` single source of truth for sensitive-name decisions
- New CLI `snapshot restore` subcommand (restores the latest or a specified snapshot); the build-failure path also writes a snapshot for recovery
- `ConfigBuilder::env_separator`/`sensitive_paths` methods; env type errors carry field paths via serde-path-to-error
- Observability: event-channel drop counter, `confers.env.path_conflict_dropped` / `confers.interpolation.sensitive_reference` telemetry events, `ProgressiveReloader::peek_candidate`

#### Changed (breaking)

- Equal-priority config sources merge in **declaration order** (previously alphabetical by source_id); the defaults source always has the lowest priority
- Prefixed env variable path conflicts (scalar vs nested) return an error instead of silently dropping values; prefix-less sources deterministically pick the "nested shape wins" + emit an event
- `load_file`/`load_file_with_env` register defaults for all `#[config(default)]` fields; `load_file_with_env` no longer injects the prefix-less whole-process environment
- `derive_field_key`/`decrypt`/`fetch_and_register` etc. return managed types (`Zeroizing`/`SecretBytes`); `SecureString::masked()` and `mask_value` switched to fixed-length masks
- The macro `#[config(dynamic)]`'s `*_handle()` is now a singleton; `#[config(merge_strategy)]` field strategies take real effect; null no longer overrides existing values (consistent at all levels)
- `user_message()` redacts across all paths; CLI `get`/`inspect` masks by default
- Sensitive files (keys.json/snapshots/audit logs) are written to disk with 0600

#### Fixed

- Single-file FsWatcher hot reload silently failing after atomic replacement; event channel full silently dropping events; callback panics killing the reload loop
- Consul empty arrays (KV deletions) permanently extending the life of stale config; K8s in-cluster missing CA/timeout; Nacos missing auth and circuit-breaker false positives; Redis bus disconnection without reconnect; bus version numbers resetting on restart
- SSRF blacklist adds `0.0.0.0/8` etc.; weak keys (all-zero, etc.) rejected; Vault token expiry auto re-login; `rotate_master_key` verifies the old key
- `verify_audit_chain` constant-time comparison; conflict report value redaction; audit sinks receive redacted events

#### Removed

- Dead dependencies `secrecy`, `aes-gcm`; `KeyCachePolicy` went from a dead type to real wiring (Vault token caching effective per NoCache/CacheWithTtl/CacheIndefinitely)

### Changed (breaking-surface declaration)

- **`#[config(validate)]` stays a compatibility no-op**: the validation helper `confers_validate()` is now generated by the new opt-in attribute `#[config(validate_helper)]` (requiring the `validation` feature and `#[derive(garde::Validate)])`); neither attribute auto-attaches validation to the load pipeline, so old code (setting only `validate`) upgrades with identical behavior and compilation
- **`CorsValidator` no longer supports unit-struct bare-name construction** (`let v: CorsValidator = CorsValidator;` fails to compile): it became fielded to support the `with_origins_key` / `with_methods_key` / `with_max_age_key` custom-key constructors; `CorsValidator::new()` and `Default` behave unchanged (default key names byte-identical to the old version); a search across the confers / mnemis / sdforge ecosystems found only `::new()` construction — no real consumers affected
- **`WatcherGuard::shutdown` exceptional-path behavior change**: when the awaited task panics (join returns `Err`), it now records the error and returns `Ok(false)`; the old version misreported panics as `Ok(true)`; timeout and clean-completion semantics unchanged
- **Log-injection protection for reload failure reasons**: reasons in `ReloadRolledBack` / `ReloadRejected`, reload logs, and canary event streams are sanitized and truncated via `flatten_reason` when they contain control characters or exceed 200 characters (control chars replaced with spaces; overflow appends an ellipsis); normal input is byte-for-byte unchanged

### Added

- `WatcherGuard::with_task` / `set_task_handle` relaxed from `pub(crate)` to `pub`; the `shutdown` docs note the degraded semantics of returning `Ok(false)` on timeout (the task is not cancelled and keeps running in the background)
- `ConfigBuilder::env_source(EnvSource)`: accepts a pre-built `EnvSource` (combination semantics with `env_prefix` / `env_separator` in rustdoc)
- The `hot-reload-kit` feature (off by default; not included in the recommended/production/full presets) + the `HotReloader` facade: an assembly of FsWatcher + progressive reload + watch broadcast + graceful shutdown; the loader is offloaded via spawn_blocking, and panics count toward the failure counter
- `ProgressiveReloader::with_pre_commit_check` + the `PreCommitCheck` trait (`Err` rejects) and the new error variant `ConfigError::ReloadRejected` (protected by `#[non_exhaustive]`)
- The `RemappedConfigProvider` key-remapping view + `JwtSecretValidator::with_secret_key` / `CorsValidator::with_{origins,methods,max_age}_key` (default key names unchanged; partial key remapping on CORS can trigger stable false positives with existing rules — see `CorsValidator` rustdoc; starting with `JwtSecretValidator` alone is recommended)

---


### Dependency Upgrades and Feature Narrowing

- Transitive patch upgrades (`cargo update`, Cargo.lock): lazy_static 1.5.0→1.5.1, pulldown-cmark-to-cmark 22.0.1→22.0.3, quinn-proto 0.11.18→0.11.19, quinn-udp 0.5.15→0.5.16, tokio-rustls 0.26.5→0.26.6, xxhash-rust 0.8.18→0.8.19, yoke-derive 0.8.3→0.8.4; direct dependencies' x.x baselines verified against crates.io as already at the latest stable major.minor — no version changes (that conclusion holds as of the rc.6 release point; afterwards etcd-client rose to 0.21 and the lock saw further patch refreshes — see the `[Unreleased]` section at the top of this file)
- Feature narrowing (`default-features = false` unchanged, explicit features minimized): `regex` `unicode` full aggregate → `unicode-case` + `unicode-perl` (all 75 static patterns in src use only `(?i)`, `\d`/`\s`/`\w`/`\b`/`\S`; the two-feature set was isolated and verified for identical compilation and byte-identical ASCII semantics); `rand` `["std","std_rng","sys_rng","thread_rng"]` → `["thread_rng"]` (the first three are its transitive subsets; the code uses only `rng()`/`Rng::fill_bytes`); examples' `notify` drops `macos_fsevent`, a no-op on Linux targets
- CI actions upgrades (SHA pins): taiki-e/install-action v2.87.20→v2.87.22, github/codeql-action v4.38.2 SHA corrected to the official tag's current target, dtolnay/rust-toolchain advanced on master (checkout/rust-cache/setup-protoc/codecov/gh-release verified as already latest and untouched)

### Decoupling of the security and encryption features (FEATURE_AUDIT_REPORT §9.2-5)

- `security` no longer implies `encryption`: the security module (`SecureString`, error redaction `ErrorSanitizer`) actually needs only three lightweight primitives — zeroize, sha2+hex (for `fingerprint`) — now declared directly by `security`; enabling `security-rules` (the rules engine is pure computational validation) and the `recommended` preset no longer pull the encryption and async stacks (chacha20poly1305/hkdf/tokio/async-trait etc.) through the chain (getrandom remains in the graph via the moka→uuid baseline chain, not removable by this decoupling; recommended direct dependencies 27→22, transitive dedup 131→119, measured per the same scope as the FAQ dependency-count table)
- **The effective public surface is unchanged**: the effective gating of exports such as `ErrorSanitizer`/`SecureString` converged from `encryption` to `security` — under the original chained semantics (security⇒encryption) both were always enabled together, so consumers notice nothing; the `--features encryption`-alone combination behaves unchanged (the security module never compiled there anyway). Combinations needing encryption enable `encryption` explicitly (the `production`/`full` presets already include it)
- Gates and docs synchronized: presets_e2e prs08 pins the decoupling negative assertion (`security` members must not contain `encryption`); the README/README_EN feature matrix `security` row rewritten; TEST_SCENARIOS §3.1 dependency-chain table and the PRS-08 row updated; API_REFERENCE `ErrorSanitizer` gating note, the FAQ `recommended` dependency-count line, and CONTRIBUTING default feature names synchronized; README/README_EN preset-table leftover old names (`env`/`schema`) corrected to the canonical names

### inklog audit two-way integration (docs-only accompaniment)

- **Two-way relationship**: inklog's existing `config-confers` feature loads config via confers and watches it hot (inklog consumes confers); this time confers' existing `audit` feature's object-safe multi-sink port (`add_sink`/`with_sink`/builder injection, see the [0.6.0-rc.3] section) is implemented by inklog's `integrations::ConfersAuditSink` (the `confers-audit` feature, pinned to confers 0.6.0-rc.5) — audit events (KeyAccess/KeyRotation/Decrypt/LoadSuccess/ReloadTrigger) keep writing the local HMAC chain file while being forwarded synchronously to inklog's structured sink (confers events flow into inklog). The port is in confers, the implementation in inklog; zero code change and zero new dependency on confers' side (no reverse dependency on inklog)
- **Integration example**: a compilable, runnable example lives in the inklog repository at `examples/src/bin/config/confers_audit.rs` (`cargo run --package inklog-examples --features confers-audit --bin confers_audit`)
- **Docs sync**: the README/README_EN feature matrix `audit` row and the USER_GUIDE "Audit Log Configuration" section note the integration

### Near-synonymous feature renaming migration (env→dotenv, key→key-management, schema→json-schema)

- Three near-synonymous feature pairs migrated to canonical names: `dotenv` (its semantics are .env file loading; `env` clashed with the `SourceKind::Environment` env-variable-source terminology), `key-management` (key rotation/version management semantics; `key` was too broad), `json-schema` (its output is JSON Schema; `schema` was too broad); the presets (default/minimal/recommended/dev/production/full/distributed) and all internal cfg gating record the canonical names
- The old names are kept one version as compatibility aliases (`env = ["dotenv"]`, `key = ["key-management"]`, `schema = ["json-schema"]`); enabling any alias emits a compile-time `cargo:warning` deprecation notice via `build.rs`, removed next version
- The migration mapping is documented: README/README_EN feature matrix notes the canonical-name and alias relations; the ARCHITECTURE feature-module mapping table, API_REFERENCE/FAQ/USER_GUIDE/LIBRARY_INTEGRATION/TEST_SCENARIOS synchronized; presets_e2e pins the alias chains (`env→dotenv`, `key→key-management`, `schema→json-schema`) and the canonical preset expansions
- Legacy test fixes: doctor's healthy assertion became feature-combination aware — degraded builds without `encryption` intentionally report a warning (exit 1); the tests pin the healthy/warning contract separately per build state, fixing the false failure of the cli suite under the `dev` preset combination

### Cloud KMS backends (cloud-kms feature: Vault + AWS + GCP)

- AWS KMS (`AwsKmsKeyProvider`): `kms:Decrypt` called directly as a SigV4-signed JSON 1.1 API (rustls HTTP, no AWS SDK) — a pure-function SigV4 implementation (hmac/sha2/hex) pinned via AWS documentation's public test vectors (the AKIDEXAMPLE vectors, not real credentials); credentials come from the builder or `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY`/`AWS_SESSION_TOKEN`, with temporary-credential support via `x-amz-security-token`; the endpoint is overridable (plain HTTP only for loopback addresses — exact matches of 127.0.0.0/8/::1/localhost; domains like 127.0.0.1.evil.com are rejected), non-loopback addresses force HTTPS
- GCP KMS (`GcpKmsKeyProvider`): `cryptoKeys.decrypt` + the instance metadata server token (the `Metadata-Flavor: Google` handshake is mandatory — that header is exactly the mechanism distinguishing real metadata traffic from SSRF probes); key resource path-component whitelist validation (traversal/escape/empty segments rejected); both metadata and KMS endpoints are overridable, and the token can be injected directly (testing/prefetched credentials)
- Both implement `CloudKmsBackend` and `AsyncKeyProvider` (reusing KeyCachePolicy); `CloudKmsBackend::decrypt` returns `ZeroizingBytes` — plaintext keys live in a zeroizing container from birth (auto-erased on drop, no plaintext copies lingering); error mapping preserves the 429/502/503/504 retryable semantics; mock-server contract tests verify request shape (server-side signature recomputation, the Metadata-Flavor handshake, the Bearer source) and decrypted bytes; real-cloud integration tests are double-gated with `#[ignore]` + the `CONFERS_AWS_KMS_LIVE` / `CONFERS_GCP_KMS_LIVE` environment variables and skip by default in sandboxes
- The `cloud-kms` feature adds `dep:hmac` / `dep:hex` (needed by SigV4); keys are never hard-coded — tests use only AWS documentation's public example constants

### Canary rollout orchestrator (canary feature = change-stream + progressive-reload)

- Added `CanaryOrchestrator` (`src/canary.rs`): cross-instance batched rollout orchestration on the ChangeStream consumer side — collects each batch's `committed` events per a `RolloutPlan` (batch_size / batch_interval / poll_interval / failure_threshold all configurable), performs batch health checks (reusing the `HealthStatus` three-state semantics: only Critical counts toward rollback decisions; Degraded is observable via the instruction detail), and publishes advance/rollback decisions back through `canary.orchestrator` instruction events (advance / rollback / completed)
- **Instance identity protocol**: instance event keys are `canary.<instance_id>` — `ProgressiveReloader` gains `with_instance_id` (default `unknown`; events without it cannot be attributed by the orchestrator); the orchestrator set-matches the instance id in the key against the current batch's instance set: duplicate `committed` events fold, unknown-instance and cross-batch events never count; a `rolled_back` from an instance in an already-advanced batch aborts the rollout immediately too (a regression signal of the candidate version on promoted batches)
- **Failure surfacing**: on the rollback path, a mesh weight push or rollback instruction publish failure → `RolloutOutcome::Aborted.mesh_rolled_back_failed = true` with the reason appended; on the advance/completion path, a mesh traffic-switch failure → best-effort switch back to baseline, then `run()` returns `Err` (the rollout never continues on an ineffective traffic switch); failures count toward `confers_canary_errors_total{reason=...}` and emit a warn under the `tracing` feature
- Three rollback triggers: `rolled_back` events from the current or advanced batches, the healthy-Critical ratio exceeding `failure_threshold` (evaluated per round, aborting immediately past the threshold without waiting for the window to exhaust), and the committed-collection timeout — all publish the rollback instruction and switch traffic back to baseline; an empty instance list (service discovery failure) errors directly, never pushing a 100% traffic switch to zero verified instances
- A single health-check call is bounded by the smaller of `poll_interval` and the window remainder (a pending implementation transitions to Degraded without blocking the rollout)
- Added the `MeshWeightPublisher` service-mesh adaptation layer: rollout progress → linear baseline↔canary traffic-weight push (`with_mesh`); Envoy weighted clusters and Istio VirtualService merge-patch config samples plus a Rust adaptation skeleton in `docs/CANARY_ORCHESTRATION.md`
- Local multi-instance simulation: an end-to-end test with multiple in-process `ProgressiveReloader`s attached to the same `InMemoryChangeStream` (`orchestrates_in_process_reloaders_end_to_end`), with 17 contract tests covering batched advance / three rollback paths (including promoted-batch regression) / Degraded / duplicate and cross-batch event dedup / empty-list fail-loud / health-check pending / both mesh failure paths / mesh weight sequences

### JSON Schema → Rust reverse generation

- CLI `schema --from-schema <FILE>`: generates compilable Rust struct scaffolding from a JSON Schema draft (2020-12) (the schema-first starting point), mutually exclusive with `--from-instance`
- Added `RustScaffoldGenerator` (the `schema` feature, exported from the `confers::schema` facade): the full mapping rules are in its rustdoc — non-required fields → `Option<T>` + `#[serde(default)]`; scalar `default` → concrete type + a generated `_default_*` default function; string enums → Rust enums (`#[serde(rename)]` keeps the original values); inline nested objects promoted to named structs; `$defs`/internal `$ref` → independent type references; free-form objects → `serde_json::Value`; `additionalProperties` subschemas → `HashMap<String, T>`; property names normalized to snake_case (`#[serde(rename)]` keeps the original key; Rust keywords use raw identifiers)
- fail-loud contract: `oneOf`/`anyOf`/`allOf`/`not` (consistent at the property layer and the `$defs` named layer), external `$ref`, internal `$ref` entries unresolvable in `$defs`, `patternProperties`, tuple arrays, and mixed non-null types all error explicitly with the field path — never silently generating wrong code; composite `default`s are marked with a `TODO` comment
- Generation safety and robustness: every schema string entering generated code (rename values, enum original values, string `default`s) is literal-escaped (quotes/backslashes/control characters), so untrusted schemas cannot inject compilable code; control characters in doc comments and `format` hints are visible-escaped as `\u{...}` text; error messages filter control characters too
- Identifier contract: property names illegal after normalization (empty, digit-leading, normalized to all underscores — including single CJK characters and other names falling only into the separator branch → `_`, with no escape via Rust reserved identifiers) and keywords that cannot be raw-ified (`self`/`super`/`crate`) error explicitly; normalization collisions (`userName`/`user-name`, enum variants `a-b`/`a_b`) get numeric suffixes plus `#[serde(rename)]` keeping the original value
- Type-semantics fixes: string enum + scalar `default` generates the matching variant path (`Enum::Variant`), erroring when the default is outside the variants; composite `default` on non-required fields stays `Option<T>` + `#[serde(default)]` (schema-optional semantics); non-required fields of nullable type arrays get a single-layer `Option` (no more double wrapping); `$defs` free-form types no longer lose the `HashMap` import; artifacts end with a single newline
- Contract tests: a new "generated artifact must compile with rustc" smoke test (covering enum×default, collisions, escaping, keyword properties, HashMap import combinations) and injection regression tests
- New example `schema_to_rust`; 20 new generator mapping contract unit tests and 4 new CLI end-to-end tests; USER_GUIDE gained a `schema` command section

### Remote sources promoted to stable (Beta → Stable)

- **Interface freeze promise**: the public APIs of `remote` / `etcd` / `consul` / `etcd-watch` are frozen from now on — incremental evolution only before 1.0 (additions only, no breaks), with breaking changes only alongside the 1.0 major. The freeze list includes all rc.6 interfaces:
  - HTTP polling: `HttpPolledSourceBuilder::stale_on_error` (off by default, keeping fail-loud), auth-header injection API, default timeout
  - etcd: all `EtcdSourceBuilder` builder methods (`endpoints` / `endpoint` / `username` / `password` / `prefix` / `format` / `interval` / `tls` / `operation_timeout` / `circuit_breaker_threshold` / `build`), `EtcdTlsConfig`, `EtcdSource::watch_transport`
  - Consul: `ConsulSourceBuilder` and its builder methods, `ConsulTlsConfig`
  - etcd watch: `WatchEventSource`, `EtcdWatcher` (`new` / `with_retry` / `last_revision` / `run`), `EtcdWatchEvent` (`key` / `value` / `mod_revision`), `EtcdWatchRetry` (`base` / `max`; defaults to a 500ms base, 30s-capped exponential backoff), `EtcdWatchCallback`, `EtcdGrpcWatchSource`
- **Added**: the `confers::async_trait` re-export (`remote` feature gated) — `WatchEventSource` is expanded via the async-trait macro; external implementations of the trait must use the same macro, and the re-export is the official implementation path
- **Runtime contract made explicit**: callbacks are invoked synchronously inline and must be non-blocking (spawn long tasks yourself), written into the `EtcdWatchCallback`/`EtcdWatcher::run` rustdoc; the resume point after reconnection is map-as-consumed (an at-most-once gap at the batch tail; consumers needing reconciliation can compare against `last_revision`); etcd values not decodable as UTF-8 are delivered lossy (`U+FFFD`) rather than erroring and redelivering — eliminating an infinite reconnect storm triggered by a single poison event; stream errors and connection failures gained tracing::warn (`tracing` feature gated) and metrics counters
- **Compatibility guarding**: new `tests/remote/etcd_watch.rs` watch compatibility regression tests — an external mock implementation drives the full watch lifecycle (events delivered in order, stream-drop reconnection, `last_revision` monotonic across connections), delete events with `value: None` passing through, frozen type shapes and default backoff semantics
- **Verification method**: the existing test matrix substitutes for a long soak — six-source feature gating (HTTP polling / etcd / etcd-watch / consul / k8s / nacos) compile verification + mock integration tests for HTTP, etcd, Consul, and watch + watch compatibility regression. Real-cluster integration tests (`tests/remote/etcd.rs` / `consul.rs`, depending on `docker-compose.test.yml`) and long soaks cannot run in a sandbox without container-registry access — an environment limitation, not a code defect
- **Baseline and gates**: added `etcd_watch_bench` (dispatching a 1000-event batch in [83.8 µs, 84.8 µs, 86.0 µs], about 0.085 µs/event); the CI benchmarks job's features widened to `dev,etcd-watch,change-stream`, so watch benchmarks (including the existing `watch_callback_bench`) are re-measured in CI from now on


## [0.6.0-rc.5] — 2026-09-21

A version-number-only release (`Cargo.toml`/`Cargo.lock` bump); contains the maintenance changes since rc.4:

- **i18n**: an in-house i18n facade (`src/i18n` catalog/locale, based on `fluent-bundle`/`unic-langid`), `LocalizedMsg` implemented for `ConfigError`, and variant-guard tests added
- **Features**: `keyring` / `cloud-kms` imply `encryption`; the `async-core` aggregate predicate introduced
- **Engineering**: pre-commit gates and a detect-secrets baseline adopted; typos word-list allowlist; `version` fields completed for path-only dependencies
- **Tests**: NATS integration test names gained nanosecond entropy to prevent cross-process collisions; leftover `AppConfig` derived names in examples/docs/tests unified to the `Confers` prefix
- **Dependencies**: removed the main crate's direct dependency `compact_str` (short-string interning was never actually wired; only an uninherited version declaration remained in `[workspace.dependencies]`)

## [0.6.0-rc.4] — 2026-09-13

A version-number-only release (`Cargo.toml`/`Cargo.lock` bump, removing the local-development dependency scope and aligning with the crates.io release chain), with no API or behavior changes; the same batch consolidated CI quality gates (18 clippy items, rustdoc private-item links, rustfmt alignment).

---

## [0.6.0-rc.3] — 2026-09-10

> This section contains two batches, `workspace-rc3-hardening` and `workspace-rc4-completion` (accumulated before the rc.3 release; version rule: target = crates.io-published rc.2 + 1).
> The version 0.6.0-rc.3 was skipped and never released (no tag, not on crates.io); this section's content shipped together with 0.6.0-rc.4.

### Added (workspace-rc4-completion)

- **Unified change-stream port** (`change-stream` feature): the `ChangeStream` trait (publish/subscribe/ack), with `InMemoryChangeStream` reusing `ConfigBus`; watch and remote changes publish a unified `ChangeEvent` envelope (key/old value/new value/source + monotonic version).
- **Real codegen consumption for four macro attributes**: `flatten` (top-level keys promoted into nested structures, with the `ConfigFieldKeys` hook), `dynamic` (the `<field>_handle()` DynamicField runtime handle), `interpolate` (field values `${key}`/`${key:default}` resolved against the merged tree), `watch` (the field-level hot-reload subscriber `field_watcher`); the four attributes combine without conflicts, asserted by macro_e2e semantics tests.
- **etcd native watch stream** (`etcd-watch` feature): `EtcdWatcher` consumes the watch stream instead of prefix GET polling, auto-reconnecting with bounded exponential backoff; the `WatchEventSource` abstraction is mockable; one-line access via `EtcdSource::watch_transport()`.
- **Kubernetes config source** (`k8s` feature): the mounted-volume source `K8sMountedSource` (aware of kubelet atomic symlink-swap writes with `..data` generation detection) + the `K8sApiSource` REST API source skeleton (ConfigMap/Secret, in-cluster detection, Secret base64 decoding).
- **Nacos config source** (`nacos` feature): HTTP Open API pulling + timed listening (unchanged content hits the cached snapshot), namespace/group mapping, circuit-breaker protection.
- **CLI `doctor` subcommand**: five checks — schema structure / source priority chain / encrypted-field decryptability (`enc:v1:` envelope + `CONFERS_MASTER_KEY`) / env override conflicts / loading; single-line JSON report + exit codes 0 healthy / 1 warning / 2 error.
- **CLI `schema --from-instance`**: reverse-derives a JSON Schema draft from a loaded config instance (object required, array items, scalar type inference).
- **tracing integration** (`tracing` feature): spans and events on four critical paths — load/reload/decrypt/remote fetch; runs in parallel with MetricsBackend without exclusion; zero overhead when the feature is off.
- **AuditSink multi-sink port**: the object-safe `pub trait AuditSink: Send + Sync`; `AuditWriter` supports injecting multiple sinks (`add_sink`/`with_sink`/builder), with the default local HMAC chain file preserved unchanged; intended for inklog to implement on top.
- **Performance baseline gates**: `watch_callback_bench` (change-stream roundtrip/fanout/accounting) + load/merge/hot_path baseline numbers recorded in `docs/PERFORMANCE.md`.
- **Agent knowledge pack**: the CLI `docs --agent` outputs subcommands/arguments/exit-code contracts/common task recipes (JSON + Markdown, ≤200 lines).
- **Macro batch renaming**: `#[config(rename_all = "camelCase|snake_case|kebab-case")]`; codegen maps external keys back to serde field names before deserialization (explicit serde names take priority), and illegal styles error at macro expansion (trybuild).
- **Cloud key backends** (`cloud-kms` feature): the Vault transit KeyProvider MVP (`POST /v1/transit/decrypt/{key}` unwraps the 32-byte master key), with the `CloudKmsBackend` port leaving AWS/GCP extension points; mock-server tests.
- **Vault auth enhancements**: `VaultAuth::AppRole` / `Kubernetes` (pod JWT + auth role) log in for a client token and cache it; `kubernetes_from_service_account` builds from in-pod token files; mock tests.
- **System keychain** (`keyring` feature): the `KeyringStore` port + `SecretToolKeyringStore` (freedesktop Secret Service) / `FileKeyringStore` (chmod-600 fallback with a warning); `MasterKeyStore::from_environment` auto-selects; tests skip in DBus-less environments.
- **OpenFeature rollout engine** (`openfeature` feature): the `FeatureProvider` port + `NoOpProvider` + `StaticFlagProvider` (attribute rules + deterministic percentage bucketing by targeting key) + `ToggleRegistryProvider` bridging the existing FeatureToggleRegistry.
- **Bus version arbitration**: `VersionArbitratedBus` wraps any `ConfigBus`; publishers stamp per-instance monotonic versions, and subscribers drop out-of-order/stale/duplicate versions with counters; unversioned events fail open.
- **Zero-copy hot path**: `InMemoryConfig`'s internal storage switched to `Arc<AnnotatedValue>`, with the new `SharedValueReader::get_shared()` shared-handle read; a ~2.1x improvement reading 10 KiB values (bench comparison recorded in PERFORMANCE.md).
- **Canary linkage** (`change-stream` feature): ProgressiveReloader phase transitions (trial_started/committed/rolled_back) publish `ChangeSource::Canary` events to the ChangeStream for upper-layer orchestration.
- **Lazy segmented parsing** (`lazy-parse` feature): `LazySegmentedConfig` splits on top-level TOML table headers (pure line scanning); segments parse and cache on first access; unit tests assert zero parsing for unvisited segments.

### Changed (workspace-rc4-completion)

- `InMemoryConfig` value storage switched to `Arc<AnnotatedValue>` (public API compatible; `get_raw`/`get_string` semantics unchanged).

### Fixed (workspace-rc3-hardening)

- **Macro env key-name override**: fixed the defect where the env key generated for `#[confers(name_env = "X")]` mismatched the declaration, ensuring custom env key names and default naming rules never pollute each other.
- **skip+default combination**: fixed the defect where `skip` fields still participated in loading and `default` was overridden by env/files; corrected the field-filtering order.

### Added (workspace-rc3-hardening)

- **Audit HMAC chain signing** (`audit` feature): `AuditEvent` computes `HMAC-SHA256(prev_hash || canonical_event_bytes)` before being written, with a random salt at the chain head; audit files write head/tail metadata; the `verify_audit_chain(path)` verification function is provided.
- **MetricsBackend critical-path instrumentation**: four path classes wired into `MetricsBackend` — loader load success/failure, watcher trigger counts, remote source fetch latency and errors, secret decryption errors; metric names prefixed `confers_`.
- **CLI `schema` subcommand**: outputs the JSON Schema of config types (needs the `cli` feature, which auto-enables `schema`).
- **CLI `get <key>` subcommand**: gets a config value by dotted path, printing single-line stable JSON; a missing key returns `null` (exit code 0).
- **CLI `--fields` global option**: trims JSON output to the specified fields (comma-separated dotted paths).
- **CLI exit-code contract**: 0 success / 1 config error / 2 I/O error.

---

## [0.6.0-rc.2] - 2026-09-07

### Changed

- Dependency upgrades: 12+ 0.x dependency refreshes (async-nats 0.50, base64 0.23, chacha20poly1305 0.11, aead 0.6 (adapting to getrandom 0.4), compact_str 0.10, garde 0.23, hkdf 0.13, sha2 0.11, aes-gcm 0.11, serde_ini 0.2, etcd-client 0.20, rand 0.10, criterion 0.8) + the darling 0.24 / syn 3.0 toolchain + indexmap/ipnet/redis refreshes
- Version bumped to `0.6.0-rc.2` (the next minor pre-release)

### Tests

- Test pyramid + E2E pinning: completed the 7 missing e2e test files with `[[test]]` registration; NATS container integration tests verified in containers (zero residue after testing); docs/TEST_SCENARIOS.md scenario pinning

### Documentation

- Install-example versions unified at 0.6.0-rc.2 (10 places); MSRV wording aligned to 1.97.1; SECURITY document CHANGELOG broken-link fix

---

## [0.5.1] - 2026-08-06

### Added

- **Security rules**: the new `SecurityValidator` trait and built-in validators (JWT secret strength, CORS config, SSRF protection, TLS settings).
- **Security validator registry**: `SecurityValidatorRegistry` with `with_defaults()`, out-of-the-box security checks.
- **Feature toggles**: the DashMap-based runtime `FeatureToggleRegistry`, thread-safe concurrent toggle management.
- **Feature toggle config loading**: the `load_from_config()` method, filling toggle state from a `ConfigProvider`.

### Fixed

- **SSRF whitelist bypass**: the `starts_with` prefix check replaced with proper URL boundary matching, preventing `https://127.0.0.1.evil.com`-style bypasses.
- **TLS version comparison**: lexicographic string comparison replaced with integer tuple comparison (fixing false positives like `"1.12" < "1.2"`).
- **Duplicate CORS violations**: the `max_age` validator switched to `else if`, avoiding duplicate violations from the same config value.
- **CORS negative-integer wraparound**: an `age >= 0` guard added, preventing negative `i64` → `u64` overflow.
- **Silent regex compilation**: illegal custom patterns now return errors instead of being silently dropped.
- **Deprecated API migration**: all `as_string()` calls in the security-rules code replaced with `as_str()`.
- **IPv6 bracket edge case**: malformed IPv6 addresses (missing the closing bracket) now report Warning instead of passing silently.
- **Environment variable mapping validation**: `validate_env_mapping` now validates both keys and values.

### Changed

- **Performance**: JWT weak-secret lookup changed from an `O(n)` linear scan to an `O(1)` `HashSet` lookup.
- **Performance**: TLS cipher-suite comparison simplified the redundant `eq_ignore_ascii_case` to a direct `==` (both sides are already uppercase).

---

## [0.5.0] - 2026-08-04

### Changed

- **Breaking - ConfigBuilder**: removed 6 unused methods: `watch()`, `validate()`, `key_provider()`, `metrics()`, `reload_strategy()`, `build_timeout()`; also removed the `ReloadStrategy` enum.
- **Breaking - KeyManager**: `new(path: PathBuf)` simplified to `new()` (no arguments).
- **Breaking - AuditConfig/AuditWriterBuilder**: removed the `durable_wal()` and `channel_size()` methods and their corresponding fields.
- **Breaking - AuditWriter**: the `write()` and `log_*()` methods now return `ConfigResult<()>` instead of `()`.
- **Breaking - TypeScriptGenerator**: `generate()` now returns `ConfigResult<String>` instead of `String`.
- **Breaking - CryptoError**: removed the `InvalidKeyLength(0)` variant and added the `KeyNotFound` variant.
- **Breaking - MergeStrategy**: the `Custom` variant's `PartialEq` now compares by name only (no longer comparing function pointers).

### Added

- **Circuit breaker**: the new `CircuitBreaker` and `CircuitState`, improving remote-source resilience.
- **Key entropy validation**: the key manager gained entropy validation.
- **Error message redaction**: prevents sensitive config values from leaking into error output.

### Fixed

- Flaky NATS JetStream test failures (added `#[serial]` for parallel tests).
- Clippy warnings: replaced literal-boolean `assert_eq!`, fixing the approx-constant warning.
- `.env` parse errors are now reported explicitly instead of being silently dropped.
- Interpolation self-reference defaults no longer falsely report circular references.
- The security module's `allowed_patterns` now take effect correctly.
- Fixed a `source_id` consistency issue in remote sources.
- `cleanup_old_keys` now filters by version number instead of the Active status.
- Removed all `eprintln` side effects from library runtime code.

### Changed

- **Performance**: the merge engine's owned-path optimization.

---

## [0.4.0] - 2026-07-03

### Changed

- **Breaking - SecureString no longer implements `Clone`**: removed `impl Clone for SecureString`, consistent with the established security posture ("prevent cloning: Clone forbidden"). This prevents sensitive material from being copied inadvertently in memory. Callers needing to share `SecureString` across threads should use `Arc<SecureString>` instead.

### Fixed

- **Interpolation nested default parsing (M1)**: the `:-` default separator is now recognized by a parser aware of `${}` nesting depth. Previously `${outer:${inner:-fallback}}` was wrongly split at the inner `:-`, breaking nested defaults.
- **URL validation false kill of `&` in query strings (M7)**: the `&` character was removed from the shell-injection dangerous-character regex. It is a standard URL query separator (`?key=val&key2=val2`). The shell logic operators `&&` and `||` are still captured by separate patterns.
- **Unbounded memory consumption in `load_env_file` (M9)**: added `MAX_ENV_FILE_SIZE` (1 MiB) and `MAX_ENV_LINE_LENGTH` (16 KiB) checks to `load_env_file`. Over-limit files are rejected before entering memory, with explanatory errors.

### Added

- **Error type separation**: errors split into the config phase (`ConfigConfigError`) and the runtime phase (`ConfersError`).
- **Factory function**: added `new_in_memory_validated()`, returning a `Result` for BrickArchitecture fail-fast initialization (later removed in favor of `InMemoryConfig::new_validated()`).
- **Backward compatibility**: added the aliases `ConfersError = ConfigError` and `ConfersResult<T>` to protect existing code.
- **SECURITY.md**: a comprehensive security policy document covering the vulnerability reporting process, security feature descriptions, best-practice guides, and the security audit process.
- **ADR-041**: async trait stabilization strategy — evaluating migration from the `async-trait` crate to native `async fn in traits`.
- **ADR-042**: source registration mechanism — the unified `SourceProvider` trait and `SourceRegistry`, supporting extensible data-source registration.
- **ADR-043**: error-type simplification strategy — layered error architecture and a simplified public API.
- **ADR-044**: test coverage target (80%) — per-module coverage requirements with CI integration.
- **ADR-045**: API versioning strategy — semver enforcement, deprecation process, and breaking-change policy.
- **BrickArchitecture compliance**: the config-phase error type `ConfigConfigError`, with 10 variants and error codes (2001-2999).

### Changed

- **Docs - README.md**: added the full feature matrix table and examples-directory section, directly linking all 13 runnable examples.
- **Docs - CONTRIBUTING.md**: added an explicit code coverage requirement (>= 80%) and updated the testing guide per ADR-044.
- **Docs - lib.rs**: added the BrickArchitecture error-separation section with migration examples.
- **Security**: the security document now covers all encryption, input validation, SSRF protection, audit logging, and key-management features.
- **Test quality - T-C-1**: audited 61 weak assertion patterns in `tests/`. Strengthened the 3 genuinely weak ones: dropped `save()` return values are now verified; bare `is_ok()` on save+list replaced with file-existence and snapshot-count checks; a dropped `build()` result replaced with explicit assertions verifying the TLS config is accepted.

---

## [0.3.0] - 2026-03-04

### 🎉 Added

#### TypeScript Schema Generation

- New `typescript-schema` feature
- Supports generating TypeScript type definitions from Rust types
- API: `confers::schema::generate_typescript::<T>()`

#### Key Management System (key feature)

- `KeyManager`: key lifecycle management
- `KeyStorage`: encrypted key storage (XChaCha20)
- `KeyRotationService`: automatic key rotation
- Key metadata and version management

#### Security Module (security feature)

- `EnvSecurityValidator`: environment variable security validation
- `ErrorSanitizer`: redaction of sensitive data in error messages
- `ConfigInjector`: secure config injection
- `SecureString`: auto-zeroizing secure string

#### Parallel Validation

- Improves validation performance for large configs

### 🔧 Changed

- Updated feature preset definitions (minimal, recommended, dev, production, full, distributed)
- Optimized XChaCha20 encryption performance
- Improved error message readability
- **Docs and codebase sync**:
  - Version numbers in all documents updated from 0.2.2 to 0.3.0
  - Fixed API method name: `load()` → `load_sync()`
  - Removed references to the nonexistent `ConfersCli` API
  - Removed nonexistent remote builder methods (`with_remote_config` etc.)
  - Remote config examples replaced with `source()` method usage
  - Updated the feature flag list to match Cargo.toml
  - Added missing feature documentation (config-bus, progressive-reload, etc.)

### 🐛 Fixed

- Fixed the `progressive-reload` feature gating (missing `async_trait`)
- Fixed the `encryption` feature test panic

### 🔒 Security

- Key management uses HKDF key derivation
- Hardened environment variable injection protection
- Added path-traversal protection

### 📊 Statistics

- New files: 17
- New code: 6,855 lines
- New features: 4 (typescript-schema, security, key, parallel)

## [0.2.2] - 2026-01-25

### Security

- Hardened internal function visibility (internal helpers changed to pub(crate))
- TlsConfig refactored to builder pattern
- Applied Clippy style fixes

### Changed

- Unified FileFormat across the codebase
- Tests: all 167 unit tests pass
- Tests: all 34 doc tests pass
- Tests: cargo clippy --all-features passes
- Tests: cargo deny check passes

## [0.2.1] - 2026-01-17

### Security

- **Enhanced audit logging system**: a full audit log with event classification, integrity protection, log rotation, and query capabilities
- **Enhanced config validation**: an advanced validation system with range, dependency, format, and consistency validators
- **Security documentation**: comprehensive security docs added to the API reference and user guide
- **Security annotations**: security annotations added to sensitive API methods (encryption, key management, audit logging, config validation)
- **Security examples**: new security examples for audit logging, config validation, and key management

### Added

- Audit event types and priorities (ConfigLoad, KeyRotation, SecurityViolation, etc.)
- Audit event generators with metadata tracking
- Audit log writers with HMAC integrity protection
- Log rotation and archiving with gzip compression
- Audit log query interface with filtering and pagination
- The AdvancedConfigValidator trait for extensible validation
- ValidationEngine executing validators by priority
- RangeFieldValidator for numeric range validation
- DependencyValidator for field dependency validation
- FormatValidator for string format validation (email, URL, etc.)
- ConsistencyValidator for cross-field consistency validation
- CachedValidationEngine with an LRU cache for performance
- Comprehensive security documentation in API_REFERENCE.md
- Security configuration best practices in USER_GUIDE.md
- Security examples in examples/src/06-encryption/ and examples/src/08-audit/
- Advanced validation example in examples/src/02-validation/02-validation-advanced_validation.rs

### Changed

- Added EncryptionError and DecryptionError to the ConfigError enum
- Added the std::io::Write import in src/audit/mod.rs
- Fixed SecureString's Arc::clone usage in src/core/loader.rs
- Updated Cargo.toml, adding the flate2 dependency for log compression
- Updated API_REFERENCE.md with comprehensive security notes
- Updated USER_GUIDE.md with security configuration best practices
- Updated examples with full security demonstrations

### Fixed

- Fixed a compile error in the `Config` derive macro caused by `Arc<SecureString>` type annotations
- Fixed unused-import warnings in src/audit/mod.rs and src/validator/mod.rs
- Fixed doc tests with correct imports and types

- Tests: all 204 tests pass (178 unit + 26 doc tests)
- Tests: all security tests pass (93)
- Tests: all audit tests pass (6)
- Tests: all validator tests pass (13)

## [0.2.0] - 2026-01-16

### Security

- **Internal implementation protection**: sensitive fields in `RemoteConfig` and `ConfigLoader` privatized against accidental exposure.
- **Sensitive data isolation**: `String` fields for sensitive data (`password`, `token`, `bearer_token`) in `HttpProvider` and `RemoteConfig` replaced with `Arc<SecureString>`.
- **Access control**: secure Builder patterns introduced for `EnvironmentValidationConfig` and `RemoteConfig`, forcing construction through `with_auth_secure` and `with_bearer_token_secure`.
- **SSRF protection**: `HttpProvider` hardened to validate URLs in all load methods (`load`, `load_sync`), preventing potential SSRF attacks even if internal state is modified.
- **Leak protection**: fixed the HTTP provider to handle `SecureString` correctly during request authentication, avoiding potential sensitive-data leaks (redacted output).
- Added a production-grade security module:
  - SecureString with automatic memory zeroing, protecting sensitive data
  - ConfigInjector for secure runtime config injection
  - InputValidator against SQL/command injection
  - ErrorSanitizer redacting sensitive data in error messages
- Added sensitive-data detection and alerts in the proc macro:
  - Detects hard-coded passwords, tokens, and private keys at compile time
  - Emits runtime warnings guiding users to safer alternatives
  - Implements input length limits against DoS attacks
- Fixed the SSRF test-mode bypass — localhost restrictions can be bypassed only in non-production environments
- Fixed environment variable injection — validation added before substitution
- Improved path-traversal protection:
  - Detects traversal patterns including URL-encoded and Windows paths
  - Blocks access to sensitive system directories (/etc, /usr, /var/log, etc.)
  - Prevents symlink attacks via canonicalization
- Enhanced the secure config injector's validation capabilities
- Refactored error redaction for better security
- Improved input validation logic

### Fixed

- Fixed the `Config` derive macro compile error when the validation feature is disabled: the `OptionalValidate` trait is now implemented automatically.
- Resolved duplicate method definitions in `ConfigLoader`.

### Added

- Created a unified file-format detection module (eliminating 4 duplicate implementations)
- Added comprehensive security tests (800+ lines of coverage)

### Changed

- Enhanced error handling with complete error types
- Optimized the HTTP provider for remote configs
- Renamed the `resource/` directory to `docs/image/` for better organization
- Updated documentation styles and links
- Removed about 120 lines of duplicated code
- Centralized format detection logic into utils/file_format.rs
- **Breaking**: default features changed from `["derive", "validation", "cli"]` to `["derive"]`, minimizing the dependency footprint
- Made `rustls` optional (now enabled only with the `remote` feature)
- Made `chrono`, `sysinfo`, and `lru` optional dependencies (moved into the `encryption` and `monitoring` features)
- Removed the unused `num_cpus` dependency
- Added feature presets simplifying configuration:
  - `minimal` - config loading only
  - `recommended` - config loading + validation
  - `dev` - full-tooling development config
  - `production` - production-ready config
  - `full` - all features enabled
- Added conditional compilation for all optional features, minimizing compile time and binary size
- Updated the `remote` feature to include `rustls`, `rustls-pki-types`, `tokio-rustls`, `failsafe`, and `base64`
- Updated the `encryption` feature to include `lru` and `chrono`
- Fixed `RefreshKind::new()` to `RefreshKind::nothing()` for sysinfo compatibility

### Dependency Updates

- All dependencies updated to the latest stable versions
- `lru` upgraded from 0.12 to 0.16.3, fixing a soundness issue (RUSTSEC-2026-0002)
- Updated core dependencies: tokio 1.48 → 1.49, serde, validator, schemars, thiserror, clap, etc.
- All 108 tests pass after the dependency updates

## [0.1.1] - 2026-01-02

### Security

- Added DNS rebinding protection to SSRF validation, preventing SSRF attacks via hostname resolution
- Added the safe_display() method to ConfigError, redacting sensitive content in error messages
- Key IDs masked in error messages to prevent sensitive-data leaks

### Fixed

- Raised the default memory cap from 10MB to 512MB, avoiding production failures
- Made the HTTP request timeout configurable (default 30s) for controllable performance
- Replaced RwLock unwrap() calls with proper error handling, avoiding panics
- Validator registry methods now return Result instead of panicking

### Added

- Added nonce cache monitoring methods (usage_percent, cache_stats) for production observability
- Added a warning in ConfigLoader for memory caps set too low (< 100MB)

### Changed

- Simplified boolean expressions in SSRF validation (Clippy improvements)
- Improved code formatting and documentation

## [0.1.0] - 2025-12-27

### Added

- Derive-macro-based type-safe configuration management
- Multi-format support (TOML, YAML, JSON, INI)
- Environment variable override support
- Built-in validation system integration
- JSON Schema generation
- File watching and hot reload support
- Encrypted storage for sensitive config
- Audit logging of config access and changes
- Remote config support (etcd, Consul, HTTP)
- Multi-command CLI tool (encrypt, validate, diff, generate, etc.)

### Changed

- Initial release
- Improved documentation and examples

### Security

- Secure memory cleanup
- AES encryption for sensitive data
- PBKDF2 key derivation

### Acknowledgments

- Thanks to all contributors and the Rust community

---

## Appendix: `confers-macros` sub-crate history

> Since 0.4.0 the `confers-macros` version evolves in lockstep with the workspace (see the repository root `Cargo.toml`); macro-related changes are recorded uniformly in this file's corresponding version entries, and the sub-crate log is no longer maintained separately. The records below predate that sub-crate's 0.4.0 and are kept for history only.

### [0.3.0] - 2026-03-04

**Added**

- Initial release
- The five derive macros `Config` / `ConfigSchema` / `ConfigMigration` / `ConfigModules` / `ConfigClap`
- Prefixed environment variable loading, defaults, sensitive-field handling
- File-based key loading (`_FILE` suffix)
- Nested config via `flatten`
- Validation integration with `garde`

[0.3.0]: https://github.com/Kirky-X/confers/releases/tag/v0.3.0

### Unreleased drafts before 0.4.0 (historical record, not item-for-item matching current code)

**Added**: security module (path-traversal protection), `SecretString` / `SecretBytes` sensitive-data protection (zeroize semantics), type-safe Schema generation, a test suite targeting 90% coverage, type-parsing performance optimization, unified error handling, API documentation and examples, user input validation, type-category enums.

**Changed**: breaking — tightened key-file path validation, `SecretString` default for sensitive fields; refactored code generation, unified derive macro API naming, compile-time performance improvements, pattern matching replacing string comparison, input validation gaining length limits and character whitelists.

**Fixed**: key-file loading path traversal, a memory leak in the type-string cache, Schema generation errors for nested `Option`, missing encryption algorithm validation, duplicate type-conversion performance issues.

**Security**: `_FILE` environment variable path-traversal protection, sensitive-data memory zeroing, user string input validation, key directory scope limits, URL-encoded traversal detection, maximum path length validation.
