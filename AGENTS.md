# Repository Guidelines

## Project Structure & Module Organization

A Rust configuration management library with a workspace structure:

- `src/` — Core library code organized by feature modules (bus/, cli/, config/, format/, remote/, security/, watcher/)
- `src/lib.rs` — Main library entry point with feature-gated module exports
- `src/cli/` — CLI binary implementation (enabled via `cli` feature)
- `macros/` — Procedural macros crate for derive macros (`#[derive(Config)]`)
- `tests/` — Integration tests (feature-gated with `#[cfg(feature = "...")]`)
- `benches/` — Criterion benchmarks for performance testing
- `examples/` — Example applications demonstrating library usage
- `docs/` — Documentation assets

Key modules in `src/`: `loader.rs`, `types.rs`, `error.rs`, `interpolation.rs`, `migration.rs`, `snapshot.rs`, `validator.rs`, `dynamic.rs`.

## Build, Test, and Development Commands

```bash
# Build with default features
cargo build

# Build with full feature set
cargo build --features full

# Run tests (default, recommended, or full features)
cargo test --features default
cargo test --features recommended
cargo test --features full

# Run specific integration test
cargo test --test integration_validation --features validation

# Run benchmarks
cargo bench --features dev

# Check code (compiles without producing binaries)
cargo check --features full

# Format code
cargo fmt --all

# Lint with clippy (warnings as errors)
cargo clippy --features full -- -D warnings

# Build documentation
cargo doc --features full --no-deps

# Security audit
cargo audit

# Run all pre-commit checks
pre-commit run --all-files
```

## Coding Style & Naming Conventions

- **Rust edition**: 2021 (MSRV: 1.81)
- **Formatting**: Standard `rustfmt` — run `cargo fmt --all` before commits
- **Linting**: Clippy with `-D warnings` — all warnings are errors
- **Naming**: Standard Rust conventions (`snake_case` for functions/variables, `PascalCase` for types)
- **Feature gates**: Use feature flags for optional functionality; modules are gated in `lib.rs`
- **Imports**: Group by external crates, then internal modules; use `use crate::` for internal paths

Pre-commit hooks enforce: trailing whitespace fixes, end-of-file fixes, YAML/TOML validation, large file checks, private key detection, and branch protection (no direct commits to main/master).

## Testing Guidelines

- **Framework**: Built-in Rust test framework with `#[test]` attributes
- **Integration tests**: Located in `tests/` directory, named `integration_*.rs`
- **Feature gating**: Tests use `#[cfg(feature = "...")]` to enable feature-specific tests
- **Test helpers**: Common test utilities in `tests/common.rs`
- **Coverage**: Minimum 80% code coverage enforced in CI (`cargo llvm-cov`)
- **Conventions**: Test functions prefixed with `test_`; use descriptive names like `test_email_validation_fail`
- **Serial tests**: Use `#[serial_test::serial]` for tests that share state

Run coverage report:
```bash
cargo llvm-cov --features full --lcov --output-path lcov.info
```

## Commit & Pull Request Guidelines

### Commit Messages
Follow conventional commits format observed in git history:
- `feat:` — New features
- `fix:` — Bug fixes
- `refactor:` — Code restructuring without behavior changes
- `chore:` — Maintenance tasks, dependency updates
- `ci:` — CI/CD changes
- `test:` — Adding or modifying tests
- `docs:` — Documentation updates

Examples: `feat(cli): add configuration wizard`, `fix: handle empty config files`, `refactor: remove dead code and improve documentation`

### Pull Requests
- CI must pass: check, test (default/recommended/full), clippy, fmt, docs, security audit
- Coverage must meet 80% minimum threshold
- PRs target `main` branch
- Ensure pre-commit hooks pass locally before pushing

## Feature Flags

The library uses extensive feature gating. Key feature presets:

| Preset | Includes |
|--------|----------|
| `default` | `toml`, `json`, `env` |
| `recommended` | `toml`, `env`, `validation`, `json` |
| `dev` | Most features for development |
| `production` | Full production feature set |
| `full` | All features |

Always test with relevant features enabled: `cargo test --features <feature>`
