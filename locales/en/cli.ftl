# confers CLI catalog (en).

cli-about = Configuration diagnostics tool for confers
cli-lang-help = UI language as a BCP-47 tag (`zh`/`en`; other well-formed tags fall back to the English catalog). Overrides CONFERS_LANG and locale detection
cli-error-prefix = Error
cli-inspect-title = Configuration Inspection
cli-inspect-loaded-sources = Loaded { $count } configuration source(s)
cli-inspect-all-keys = All configuration keys:
cli-inspect-requested-keys = Requested keys:
cli-col-key = KEY
cli-col-value = VALUE
cli-col-source = SOURCE
cli-col-location = LOCATION
cli-location-line-col = line { $line }, col { $col }
cli-not-found = [NOT FOUND]
cli-reveal-warning = warning: --reveal is set: sensitive values are printed verbatim
cli-snapshot-none-found = No snapshots found in { $directory }
cli-snapshot-file-missing = Snapshot file does not exist: { $file }
cli-snapshot-runtime-build-failed = runtime build failed: { $message }
cli-snapshot-restored = restored: { $file }
cli-snapshot-top-level-keys = top-level keys: { $count }
cli-schema-absolute-path-not-allowed = Absolute path not allowed: { $path }. Use --allow-absolute-paths to override.
cli-schema-read-failed = Failed to read schema: { $path }
cli-schema-parse-failed = Failed to parse schema: { $path }
cli-doctor-encryption-not-compiled = encryption feature not compiled in; encrypted values will not be decrypted at load time
cli-docs-global-args-lang = --lang <bcp47> (zh/en; others fall back to en)

# --- clap help / about (Cli, Commands, SnapshotCommands) ---

cli-config-help = Configuration file(s) to load
cli-env-file-help = Additional environment file
cli-allow-absolute-paths-help = Allow absolute paths for config files (use with caution, mainly for testing)
cli-fields-help = Filter output to only include specified fields (comma-separated dot-paths, e.g. "a.b,c.d")
cli-inspect-help = Inspect configuration - list all keys with their sources
cli-inspect-key-help = Show only specific key(s)
cli-inspect-show-conflicts-help = Show all conflicts (values that were overridden)
cli-inspect-format-help = Output format (text, json)
cli-inspect-reveal-help = Reveal values of sensitive-looking keys (default: masked)
cli-validate-help = Validate configuration against schema
cli-validate-strict-help = Strict mode: treat warnings as errors
cli-validate-format-help = Output format (text, json)
cli-export-help = Export merged configuration (sanitized)
cli-export-format-help = Output format (json, toml, yaml)
cli-export-output-help = Output file (default: stdout)
cli-export-with-provenance-help = Include provenance (source) information
cli-export-raw-help = Do not sanitize sensitive values
cli-diff-help = Diff two configurations
cli-diff-base-help = Base configuration file
cli-diff-overlay-help = Overlay configuration file
cli-diff-format-help = Output format (text, json)
cli-diff-sanitize-help = Sanitize sensitive values in output
cli-snapshot-help = Manage configuration snapshots
cli-snapshot-list-help = List all snapshots
cli-snapshot-list-directory-help = Directory containing snapshots
cli-snapshot-diff-help = Diff between two snapshots
cli-snapshot-diff-latest-help = Number of recent snapshots to compare
cli-snapshot-diff-directory-help = Directory containing snapshots
cli-snapshot-prune-help = Prune old snapshots
cli-snapshot-prune-older-than-help = Keep snapshots newer than this (e.g., "30d", "7d")
cli-snapshot-prune-directory-help = Directory containing snapshots
cli-snapshot-restore-help = Restore (load and validate) a snapshot file (disaster recovery)
cli-snapshot-restore-file-help = Snapshot file to restore; defaults to the newest snapshot in the directory when omitted
cli-snapshot-restore-directory-help = Directory containing snapshots
cli-schema-help = Output JSON Schema for the configuration type
cli-schema-from-instance-help = Infer a schema draft from the loaded configuration instance instead of the derived type (schema-first reverse engineering)
cli-schema-from-schema-help = Generate Rust struct scaffolding from a JSON Schema draft file (2020-12 flavored; non-required fields become Option, scalar defaults become generated default fns, string enums become Rust enums; unsupported constructs fail loudly)
cli-get-help = Get a specific configuration value by key path (dot-separated)
cli-get-key-help = Configuration key path (e.g., "server.host" or "database.pool.size")
cli-get-reveal-help = Reveal values of sensitive-looking keys (default: masked)
cli-docs-help = Documentation output. `--agent` emits the machine-readable knowledge pack (subcommands, arguments, exit-code contract, task recipes)
cli-docs-agent-help = Emit the agent knowledge pack
cli-docs-format-help = Knowledge format (json, markdown)
cli-doctor-help = Diagnose configuration health (schema, source chain, encryption, env overrides) and print a single-line JSON report
cli-doctor-format-help = Output format (json = single line, text)

# --- env-file loading errors ---

cli-env-file-not-found = Environment file not found: { $path }
cli-env-file-stat-failed = Failed to stat env file: { $path }
cli-env-file-too-large = Environment file too large: size limit { $limit } bytes exceeded (file is { $size } bytes): { $path }
cli-env-file-read-failed = Failed to read env file: { $path }
cli-env-file-line-too-long = Environment file line too long: limit { $limit } bytes exceeded (line is { $size } bytes): { $path }

# --- validate ---

cli-unsupported-format = Unsupported format: { $format }
cli-validate-title = Configuration Validation
cli-validate-loaded-ok = ✓ Configuration loaded successfully
cli-validate-issues-found = ✗ Found { $count } validation issue(s):
cli-validate-all-passed = ✓ All validation checks passed
cli-validate-strict-failed = Validation failed with { $count } issue(s)
cli-validate-error = ✗ Configuration error: { $message }
cli-validate-failed = Validation failed

# --- export --raw ---

cli-export-raw-warning = ⚠️  WARNING: the --raw option exports unsensitized sensitive data!
cli-export-raw-warning-detail = Make sure the output target is secure; avoid leaking passwords, keys, and other sensitive information.

# --- diff ---

cli-diff-title = Configuration Diff
cli-diff-pair = { $base } vs { $overlay }
cli-diff-identical = Configurations are identical
cli-diff-differ = Configurations differ
cli-diff-base-heading = Base ({ $path }):
cli-diff-overlay-heading = Overlay ({ $path }):
cli-diff-unified = Unified Diff:
cli-diff-read-base-failed = Failed to read base config: { $path }
cli-diff-read-overlay-failed = Failed to read overlay config: { $path }
cli-diff-base-format-unknown = Unknown format for base config
cli-diff-overlay-format-unknown = Unknown format for overlay config

# --- snapshot ---

cli-snapshot-dir-missing = Snapshot directory does not exist: { $directory }
cli-snapshot-in-dir = Snapshots in { $directory }:
cli-col-timestamp = TIMESTAMP
cli-col-filename = FILENAME
cli-col-format = FORMAT
cli-snapshot-diff-need-two = Need at least 2 snapshots to diff, found { $count }
cli-snapshot-diff-between = Diff between { $first } and { $second }
cli-snapshot-prune-invalid-duration = invalid duration '{ $value }': expected a number of days with optional 'd' suffix (e.g., '30d')
cli-snapshot-prune-summary = Pruned { $removed } snapshot(s) older than { $days } days ({ $failed } failed, { $skipped } skipped)

# --- doctor ---

cli-doctor-load-ok = configuration loaded from { $count } file(s)
cli-doctor-load-failed = configuration failed to load: { $message }
cli-doctor-skipped-not-loaded = skipped: configuration did not load
cli-doctor-schema-ok = no schema issues detected
cli-doctor-schema-issues = { $count } schema issue(s): { $issues }
cli-doctor-sources-none = no configuration values were contributed by any source
cli-doctor-sources-drifted = source(s) with inconsistent priorities across keys: { $sources } { $example }
cli-doctor-sources-env-below-file = env priority ({ $env }) is not above the file priority ({ $file }); env overrides would lose
cli-doctor-sources-ok = priority chain coherent across source(s): { $sources }
cli-doctor-encryption-none = no encrypted fields detected
cli-doctor-encryption-no-key = { $count } encrypted field(s) found but { $env_var } is not set
cli-doctor-encryption-invalid-key = { $env_var } is not a valid 32-byte hex key; encrypted field(s) cannot be decrypted
cli-doctor-encryption-ok = { $count } encrypted field(s) decrypt successfully
cli-doctor-encryption-failed = decryption failed for { $count } encrypted field(s): { $fields }
cli-doctor-env-overrides-ok = no conflicting env overrides
cli-doctor-env-overrides-conflicts = { $count } env override conflict(s): { $conflicts }
cli-doctor-header = Doctor (status: { $status }, exit: { $code })

# --- agent knowledge pack (docs --agent) ---

cli-docs-subcommand-inspect-purpose = list keys with source+location provenance
cli-docs-subcommand-validate-purpose = structural sanity checks (nulls, number-shaped strings)
cli-docs-subcommand-export-purpose = print merged config (sanitized by default)
cli-docs-subcommand-diff-purpose = diff two config files
cli-docs-subcommand-snapshot-purpose = manage snapshots
cli-docs-subcommand-schema-purpose = JSON Schema for the config type, or Rust scaffolding from a draft
cli-docs-subcommand-get-purpose = single key lookup, single-line JSON
cli-docs-subcommand-doctor-purpose = health report: schema, source chain, encryption, env conflicts
cli-docs-subcommand-docs-purpose = this knowledge pack
cli-docs-global-args-config-file = --config <file> (repeatable)
cli-docs-args-from-instance = --from-instance (reverse-engineer a draft from the loaded instance)
cli-docs-args-from-schema = --from-schema <file> (generate Rust struct scaffolding from a JSON Schema draft)
cli-docs-exit-0 = success (or doctor: healthy)
cli-docs-exit-1 = configuration error / doctor has warnings
cli-docs-exit-2 = I/O error / doctor has errors
cli-docs-contract-doctor = single-line JSON: { $shape }; severity ok|warning|error
cli-docs-contract-get = single-line JSON
cli-docs-contract-schema = pretty-printed JSON Schema (draft 2020-12 flavor for --from-instance); --from-schema prints Rust source scaffolding
cli-docs-recipe-load-env-overlay = load config with env overlay
cli-docs-recipe-ci-gate = quality gate a config in CI
cli-docs-recipe-find-source = find where a value comes from
cli-docs-recipe-get-value = get one value
cli-docs-recipe-schema-reverse = schema-first reverse engineering
cli-docs-recipe-schema-adopt = schema-first adoption
cli-docs-recipe-export-sanitized = export sanitized config
cli-docs-markdown-title = # confers agent knowledge pack
cli-docs-markdown-knowledge-version = knowledge_version: { $version }
cli-docs-markdown-subcommands-heading = ## Subcommands
cli-docs-markdown-exit-codes-heading = ## Exit codes
cli-docs-markdown-recipes-heading = ## Recipes
cli-docs-markdown-subcommand = - **{ $name }** — { $purpose } (args: { $args })
cli-docs-markdown-exit-0 = - `0` success (doctor: healthy)
cli-docs-markdown-exit-1 = - `1` config error / doctor warnings
cli-docs-markdown-exit-2 = - `2` I/O error / doctor errors
cli-docs-nothing-to-show = nothing to show: use `confers docs --agent` for the knowledge pack
cli-export-wrote-file = Exported configuration to: { $path }
cli-export-wrote-annotated = Exported annotated configuration to: { $path }
cli-snapshot-none-after-sort = No snapshots found after sorting
cli-snapshot-diff-not-enough = Not enough snapshots to compare
cli-validate-missing-server-section = Server configuration missing host/port
cli-validate-missing-database-section = Database configuration missing connection details
cli-validate-null-value-indexed = Configuration key '{ $key }[{ $index }]' has null value
cli-validate-null-value = Configuration key '{ $key }' has null value
cli-validate-string-like-number = Key '{ $key }' has string value that looks like a number: { $value }
cli-validate-string-like-boolean = Key '{ $key }' has string value that looks like boolean: { $value }
