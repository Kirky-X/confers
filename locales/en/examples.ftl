# confers examples catalog (en). Mirrors locales/zh/examples.ftl key-for-key.

# --- audit ---

example-audit-title = Audit - audit log demo
example-audit-log-dir = Log directory: { $path }
example-audit-build-config = [Build AuditConfig]
example-audit-build-writer = [Build AuditWriter]
example-audit-level-mapping = [AuditLevel event mapping]
example-audit-event-level = event { $index } -> { $level }
example-audit-write-events = [Write audit events]
example-audit-wrote-events = Wrote 6 audit events
example-audit-log-contents = [Audit log contents] { $path }
example-audit-log-line = line { $index }: { $line }
example-audit-log-empty = (log file is empty)
example-audit-read-log-failed = Failed to read log: { $error }
example-audit-completed = Example completed!

# --- basic_usage ---

example-basic-usage-title = Basic Usage - basic config loading demo
example-basic-usage-config-info = ========== Configuration info ==========
example-basic-usage-server-section = [Server configuration]
example-basic-usage-host = host: { $host }
example-basic-usage-port = port: { $port }
example-basic-usage-workers = workers: { $workers }
example-basic-usage-database-section = [Database configuration]
example-basic-usage-max-connections = Max connections: { $count }
example-basic-usage-feature-section = [Feature configuration]
example-basic-usage-logging-enabled = logging enabled: { $enabled }
example-basic-usage-metrics-enabled = metrics enabled: { $enabled }
example-basic-usage-log-level = log level: { $level }
example-basic-usage-app-metadata = [App metadata]
example-basic-usage-app-name = name: { $name }
example-basic-usage-app-version = version: { $version }
example-basic-usage-port-zero = server port must not be 0
example-basic-usage-workers-zero = worker count must not be 0
example-basic-usage-max-connections-zero = max connections must not be 0
example-basic-usage-invalid-log-level = invalid log level: { $level } (valid values: { $valid }
example-basic-usage-cwd = current working directory: { $cwd }
example-basic-usage-env-section = [Environment variable override]
example-basic-usage-env-hint = Override configuration values with these environment variables:
example-basic-usage-loading = Loading configuration...
example-basic-usage-load-failed = configuration load failed: { $error }
example-basic-usage-validating = Validating configuration...
example-basic-usage-validation-failed = Configuration validation failed: { $error }
example-basic-usage-validation-passed = ✓ Configuration validation passed!
example-basic-usage-ready = Configuration loaded and validated; the application can start...
example-basic-usage-server-address = server address: { $host }:{ $port }
example-basic-usage-logging-level = logging enabled, level: { $level }
example-basic-usage-metrics-on = metrics enabled
example-basic-usage-completed = Example completed!

# --- config_bus ---

example-config-bus-title = ConfigBus config change broadcast demo
example-config-bus-bus-created = ✓ InMemoryBus created (capacity 256)
example-config-bus-subscriber-a-subscribed = ✓ Subscriber A subscribed
example-config-bus-event-published-file-watcher = ✓ Event published: source=file_watcher, keys=[server.port, server.host]
example-config-bus-subscriber-a-received = ✓ Subscriber A received event: instance={ $instance }, source={ $source }, checksum={ $checksum }
example-config-bus-multi-subscriber = --- Demo: multi-subscriber broadcast ---
example-config-bus-subscribers-bc-subscribed = ✓ Subscribers B and C subscribed (current subscriber count: { $count })
example-config-bus-event-published-env-override = ✓ Event published: source=env_override
example-config-bus-subscriber-b-received = ✓ Subscriber B received: source={ $source }, keys={ $keys }
example-config-bus-subscriber-c-received = ✓ Subscriber C received: source={ $source }, keys={ $keys }
example-config-bus-create-directly = --- Demo: create InMemoryBus directly ---
example-config-bus-received-via-in-memory-bus = ✓ Received via InMemoryBus: source={ $source }, checksum={ $checksum }
example-config-bus-completed = Example completed

# --- cli_integration ---

example-cli-integration-host-description = Server listening address
example-cli-integration-port-description = Server listening port
example-cli-integration-workers-description = Number of worker threads
example-cli-integration-log-level-description = Log level (trace|debug|info|warn|error)
example-cli-integration-debug-description = Enable debug mode
example-cli-integration-override-mapping = [CLI override mapping (to_config_map)]
example-cli-integration-override-empty = (empty)
example-cli-integration-title = CLI integration demo - ConfigClap derive macro
example-cli-integration-how-to-run = How to run:
example-cli-integration-step1 = === Step 1: Default configuration ===
example-cli-integration-default-config = Default config
example-cli-integration-step2 = === Step 2: Parse CLI arguments (clap_args) ===
example-cli-integration-parsing-result = [CLI parsing result]
example-cli-integration-step3 = === Step 3: Convert to override mapping (to_config_map) ===
example-cli-integration-step4 = === Step 4: Merge configuration (CLI overrides defaults) ===
example-cli-integration-merged-config = Merged config
example-cli-integration-completed = Example completed

# --- context_aware ---

example-context-aware-title = Context-Aware - context-aware config demo
example-context-aware-value-types = [ContextValue types]
example-context-aware-string = String: { $value } (as_str={ $as_str })
example-context-aware-number = Number: { $value } (as_number={ $as_number })
example-context-aware-bool = Bool: { $value } (as_bool={ $as_bool })
example-context-aware-build-upload-limit = [Build context-aware fields - upload limit]
example-context-aware-free-plan = free plan: 100MB
example-context-aware-pro-plan = pro plan: 1GB
example-context-aware-enterprise-plan = enterprise plan: 10GB
example-context-aware-resolve-by-plan = [Resolve by plan]
example-context-aware-free-bytes = free: { $bytes } bytes
example-context-aware-pro-bytes = pro: { $bytes } bytes
example-context-aware-enterprise-bytes = enterprise: { $bytes } bytes
example-context-aware-resolve-log-level = [Resolve log level by environment]
example-context-aware-resolve-endpoints = [Resolve API endpoints by region]
example-context-aware-resolve-feature-flags = [Resolve feature flags by target key]
example-context-aware-rule-description = [ContextRule description]
example-context-aware-rule-desc-text = returns 100 when the tier attribute is present
example-context-aware-rule-matches = rule matches: { $matches }
example-context-aware-rule-value = rule value: { $value }
example-context-aware-completed = Example completed!

# --- snapshot ---

example-snapshot-title = config snapshot persistence demo
example-snapshot-in-memory-created = ✓ In-memory config created with server.host and server.port
example-snapshot-manager-created = ✓ Snapshot manager created; output directory: { $dir }
example-snapshot-saved = ✓ Snapshot saved: { $file }
example-snapshot-redacted = ✓ Sensitive field password redacted to [REDACTED]
example-snapshot-list-header = --- Snapshot list ({ $count } total)---
example-snapshot-list-row = { $name } | { $size } bytes | { $created }
example-snapshot-cleanup-demo = --- Demo: snapshot cleanup (max_snapshots=5)---
example-snapshot-cleanup-removed = ✓ Manual cleanup removed { $count } old snapshots
example-snapshot-remaining = Remaining snapshots: { $count }
example-snapshot-completed = Example completed

# --- progressive_reload ---

example-progressive-reload-title = progressive reload demo
example-progressive-reload-strategy-immediate = --- Strategy 1: Immediate (immediate reload)---
example-progressive-reload-initial-config = Initial config: port={ $port }, host={ $host }
example-progressive-reload-current-config = current configuration: port={ $port }, host={ $host }
example-progressive-reload-strategy-canary = --- Strategy 2: Canary (canary rollout)---
example-progressive-reload-initial-port = Initial config: port={ $port }
example-progressive-reload-current-port = Current config: port={ $port }
example-progressive-reload-strategy-linear = --- Strategy 3: Linear (linear rollout)---
example-progressive-reload-completed = Example completed
example-progressive-reload-outcome-committed = ✓ Reload result: Committed (committed)
example-progressive-reload-outcome-rolled-back = ✗ Reload result: RolledBack (rolled back) - { $reason }

# --- modules_demo ---

example-modules-demo-title = Modules - configuration module system demo
example-modules-demo-conf-dir = Configuration directory: { $dir }
example-modules-demo-create-registry = [Create module registry]
example-modules-demo-registry-size = Registry size: { $size }
example-modules-demo-is-empty = Is empty: { $empty }
example-modules-demo-register-groups = [Register configuration groups]
example-modules-demo-registered-groups = [Registered groups]
example-modules-demo-group-row = Group: { $group } (active configuration: { $active })
example-modules-demo-details-database = [Module configuration details - database]
example-modules-demo-module-name = Module name: { $name }
example-modules-demo-active-profile = Active configuration: { $profile }
example-modules-demo-profile-count = Configuration count: { $count }
example-modules-demo-available-profiles = Available configurations: { $profiles }
example-modules-demo-has-mysql = Has mysql: { $has }
example-modules-demo-load-module = [Load module configuration]
example-modules-demo-loaded-postgresql = database/postgresql loaded successfully
example-modules-demo-load-active = [Load active configuration]
example-modules-demo-loaded-redis = cache active configuration (redis) loaded successfully
example-modules-demo-switch-active = [Switch active configuration]
example-modules-demo-active-before-switch = database active before switch: { $profile }
example-modules-demo-active-after-switch = database active after switch: { $profile }
example-modules-demo-all-active = [All active configurations]
example-modules-demo-env-parsing = [Environment variable parsing]
example-modules-demo-active-after-env = database active after environment variable resolution: { $profile }
example-modules-demo-validate-active = [Validate active configurations]
example-modules-demo-all-files-exist = All active configuration files exist
example-modules-demo-validation-failed = Validation failed: { $error }
example-modules-demo-completed = Example completed!

# --- schema_to_rust ---

example-schema-to-rust-title = JSON Schema -> Rust scaffolding demo
example-schema-to-rust-completed = Example completed
example-schema-to-rust-basic-mapping = --- Basic mapping ---
example-schema-to-rust-defs-and-refs = --- $defs and internal $ref ---
example-schema-to-rust-explicit-error = --- Explicit error for unsupported structures ---
example-schema-to-rust-unreachable = (unreachable)
example-schema-to-rust-failed-as-expected = Failed as expected: { $error }

# --- interpolation ---

example-interpolation-title = Interpolation - variable interpolation demo
example-interpolation-basic = [Basic interpolation]
example-interpolation-template = template: { $template }
example-interpolation-result = result: { $result }
example-interpolation-default-values = [default values]
example-interpolation-nested-references = [nested references]
example-interpolation-nesting-in-defaults = [Nesting in default values]
example-interpolation-traced-normal = [Traced interpolation - normal field]
example-interpolation-value = value: { $value }
example-interpolation-referenced-variables = referenced variables: { $variables }
example-interpolation-is-sensitive-field = is sensitive field: { $sensitive }
example-interpolation-traced-sensitive = [Traced interpolation - sensitive field]
example-interpolation-contains-sensitive-refs = contains sensitive references: { $contains }
example-interpolation-config = [Interpolation config]
example-interpolation-api-key-sensitive = API_KEY is sensitive: { $sensitive }
example-interpolation-db-password-sensitive = DB_PASSWORD is sensitive: { $sensitive }
example-interpolation-host-sensitive = HOST is sensitive: { $sensitive }
example-interpolation-batch-context = [Batch interpolation context]
example-interpolation-api-key-sensitive-ref = API_KEY is sensitive reference: { $sensitive }
example-interpolation-host-sensitive-ref = HOST is sensitive reference: { $sensitive }
example-interpolation-api-key-ref-field = API_KEY referenced from field: { $field }
example-interpolation-warnings = [Interpolation warnings]
example-interpolation-warning = warning: { $warning }
example-interpolation-unresolved-errors = [Unresolved variable errors]
example-interpolation-unreachable = unreachable here: { $value }
example-interpolation-expected-error = expected error: { $error }
example-interpolation-completed = Example completed!

# --- security ---

example-security-title = Security - security module demo
example-security-encryption-prefix-basics = [EncryptionPrefix basics]
example-security-prefix-string = prefix string: { $prefix }
example-security-value-encrypted = value { $value } is encrypted: { $encrypted }
example-security-strip-prefix = [Strip encryption prefix]
example-security-no-prefix = no prefix
example-security-no-prefix-plaintext = { $value } no prefix (plaintext)
example-security-display-fromstr = [Display and FromStr]
example-security-parse-failed = parse failed
example-security-parse-failed-expected = FromStr("invalid") -> parse failed (expected)
example-security-validator-defaults = [EnvSecurityValidator defaults]
example-security-app-port-allowed = APP_PORT allowed: { $allowed }
example-security-path-allowed = PATH allowed: { $allowed }
example-security-api-key-allowed = api_key (lowercase) allowed: { $allowed }
example-security-validate-names = [Validate environment variable names]
example-security-name-valid = { $name } -> valid
example-security-name-invalid = { $name } -> invalid: { $error }
example-security-validate-values = [Validate environment variable values]
example-security-value-valid = { $value } -> valid
example-security-value-invalid = { $value } -> invalid: { $error }
example-security-encrypted-bypass = [Encrypted value bypasses validation]
example-security-encrypted-value-passes = encrypted value passes value validation
example-security-encrypted-value-rejected = encrypted value rejected: { $error }
example-security-api-key-passes = API_KEY(encrypted value) passes name validation
example-security-api-key-rejected = API_KEY rejected: { $error }
example-security-custom-config = [Custom validation configuration]
example-security-name-64-valid = 64-character name valid: { $valid }
example-security-name-65-valid = 65-character name valid: { $valid }
example-security-strict-lenient = [Strict and lenient validators]
example-security-log-redaction = [Log redaction]
example-security-original-length = Original length: { $length }
example-security-redacted-length = Redacted length: { $length } (ends with ...)
example-security-completed = Example completed!

# --- remote_consul ---

example-remote-consul-title = Remote Consul config source demo
example-remote-consul-demo1 = === Demo 1: Basic Consul configuration polling ===
example-remote-consul-built = ConsulSource built successfully
example-remote-consul-source-id = source ID: { $id }
example-remote-consul-poll-interval = poll interval: { $interval }
example-remote-consul-polling = Polling Consul KV (prefix: myapp/config)...
example-remote-consul-loaded = ✓ config loaded successfully
example-remote-consul-value-map = value type: Map (multiple key-value pairs)
example-remote-consul-value-null = value type: Null (no configuration under the prefix)
example-remote-consul-value-scalar = value type: scalar value
example-remote-consul-poll-failed-normal = ✗ Poll failed (it is normal if Consul is not running)
example-remote-consul-error = error: { $error }
example-remote-consul-hint = Hint: run the following command to start Consul:
example-remote-consul-demo2 = === Demo 2: Connection with ACL token ===
example-remote-consul-with-token-built = ConsulSource with token built successfully
example-remote-consul-loaded-token-verified = ✓ Config loaded successfully (token verified)
example-remote-consul-poll-failed = ✗ Poll failed: { $error }
example-remote-consul-token-note = Note: always use HTTPS + ACL token in production
example-remote-consul-demo3 = === Demo 3: Builder parameters and security limits ===
example-remote-consul-security-limits = ConsulSource security limits configuration:
example-remote-consul-max-response-body = Max response body: 4 MB
example-remote-consul-max-kv-entries = Max KV entries: 5000
example-remote-consul-dos-limits-note = These limits prevent DoS/OOM from oversized responses (CWE-400/CWE-502)
example-remote-consul-address = Consul address: { $address }
example-remote-consul-completed = Example completed

# --- remote_etcd ---

example-remote-etcd-title = Remote etcd config source demo
example-remote-etcd-demo1 = === Demo 1: Basic etcd configuration polling ===
example-remote-etcd-built = EtcdSource built successfully
example-remote-etcd-source-id = source ID: { $id }
example-remote-etcd-poll-interval = poll interval: { $interval }
example-remote-etcd-build-failed-normal = ✗ EtcdSource build failed (it is normal if etcd is not running)
example-remote-etcd-error = error: { $error }
example-remote-etcd-hint = Hint: run the following command to start etcd:
example-remote-etcd-polling = Polling etcd KV (prefix: myapp/config)...
example-remote-etcd-loaded = ✓ config loaded successfully
example-remote-etcd-value-map = value type: Map (multiple key-value pairs)
example-remote-etcd-value-null = value type: Null (no configuration under the prefix)
example-remote-etcd-value-scalar = value type: scalar value
example-remote-etcd-poll-failed-normal = ✗ Poll failed (it is normal if etcd is not running)
example-remote-etcd-demo2 = === Demo 2: Authenticated etcd connection ===
example-remote-etcd-with-auth-built = EtcdSource with authentication built successfully
example-remote-etcd-poll-failed = ✗ Poll failed: { $error }
example-remote-etcd-build-failed = ✗ build failed: { $error }
example-remote-etcd-credentials-note = Note: In production, read credentials from environment variables instead of hard-coding them
example-remote-etcd-demo3 = === Demo 3: TLS configuration and multi-endpoint cluster ===
example-remote-etcd-with-tls-built = EtcdSource with TLS built successfully
example-remote-etcd-build-failed-unreachable = ✗ build failed (unreachable remote endpoints are normal)
example-remote-etcd-tls-steps = Production TLS configuration steps:
example-remote-etcd-tls-step-1 = 1. Generate the CA certificate and key
example-remote-etcd-tls-step-2 = 2. Generate certificates for the etcd server
example-remote-etcd-tls-step-3 = 3. Generate certificates for the client
example-remote-etcd-tls-step-4 = 4. Enable TLS when starting etcd (--cert-file, --key-file, --client-cert-auth)
example-remote-etcd-tls-step-5 = 5. Connect the client through the TLS endpoint
example-remote-etcd-endpoint = etcd endpoint: { $endpoint }
example-remote-etcd-completed = Example completed

# --- migration ---

example-migration-title = === Configuration version migration example ===
example-migration-version-chain = Version: V1 -> V2 -> V3
example-migration-all-completed = === All examples completed ===
example-migration-basic-demo = --- Basic migration example ---
example-migration-execute = Execute migration: v{ $from } -> v{ $to }
example-migration-succeeded = Migration succeeded! version: { $from } -> { $to }
example-migration-version-field = version field after migration: { $version }
example-migration-failed = Migration failed: { $error }
example-migration-path-demo = --- Path precomputation example ---
example-migration-migrate-1-2 = Migrate 1 -> 2
example-migration-migrate-2-3 = Migrate 2 -> 3
example-migration-migrate-1-3-direct = Migrate 1 -> 3 directly
example-migration-path-1-3 = Migration path (1 -> 3): { $path }
example-migration-path-1-2 = Migration path (1 -> 2): { $path }
example-migration-path-2-3 = Migration path (2 -> 3): { $path }
example-migration-optimal-path = Found the optimal path: { $path }
example-migration-multi-step-demo = --- Multi-step migration example ---
example-migration-step-1 = [Step 1] Add the database configuration
example-migration-step-2 = [Step 2] Add the cache and security configuration
example-migration-initial-version = Initial configuration version: { $version }
example-migration-multi-step-succeeded = Multi-step migration succeeded!
example-migration-final-version = Final version: { $version }
example-migration-config-keys = Configuration keys: { $keys }
example-migration-on-reload-demo = --- MigrationOnReload example ---
example-migration-always-desc = MigrationOnReload::Always: run the migration on every reload
example-migration-on-version-change-desc = MigrationOnReload::OnVersionChange: migrate only when the version changes (default)
example-migration-disabled-desc = MigrationOnReload::Disabled: migrate when reload is disabled
example-migration-current-always = Current configuration: Always
example-migration-current-on-version-change = Current configuration: OnVersionChange (recommended)
example-migration-current-disabled = Current configuration: Disabled
example-migration-hot-reload-usage = Used in hot-reload scenarios:
example-migration-auto-migrate-comment = // migrate automatically when the configuration changes

# --- dynamic_fields ---

example-dynamic-fields-start = === DynamicField example start ===
example-dynamic-fields-all-completed = === All examples completed ===
example-dynamic-fields-basic-demo = --- 1. Basic usage demo ---
example-dynamic-fields-initial-value = Create DynamicField, initial value: { $value }
example-dynamic-fields-lock-free-read = Lock-free read: { $value }
example-dynamic-fields-after-update = Value after update: { $value }
example-dynamic-fields-callback-guard-demo = --- 2. CallbackGuard lifecycle management ---
example-dynamic-fields-initial-payload = initial value
example-dynamic-fields-callbacks-before = Callback count before registering: { $count }
example-dynamic-fields-callback-fired = [callback fired] value changed to: { $value }
example-dynamic-fields-callbacks-after = Callback count after registering: { $count }
example-dynamic-fields-first-update = first update
example-dynamic-fields-second-update = second update
example-dynamic-fields-callback-times = Callback fired times: { $count }
example-dynamic-fields-callbacks-after-drop = Callback count after dropping the guard: { $count }
example-dynamic-fields-builder-demo = --- 3. Builder pattern ---
example-dynamic-fields-builder-value = Field value created with Builder: { $value }
example-dynamic-fields-default-value = Field value created with Default: { $value }
example-dynamic-fields-perf-demo = --- 4. Performance comparison demo ---
example-dynamic-fields-lock-free-time = Lock-free read (DynamicField) { $iterations } iterations took: { $duration }
example-dynamic-fields-rwlock-time = RwLock read { $iterations } iterations took: { $duration }
example-dynamic-fields-speedup = Lock-free reads are { $speedup }x faster than RwLock
example-dynamic-fields-multi-callback-demo = --- 5. Multiple callbacks listening at once ---
example-dynamic-fields-current-callbacks = Current callback count: { $count }
example-dynamic-fields-after-update-10 = After updating the value to 10:
example-dynamic-fields-callback-a-value = callback A accumulated value: { $value }
example-dynamic-fields-callback-b-value = callback B accumulated value: { $value }
example-dynamic-fields-callback-c-value = callback C accumulated value: { $value }
example-dynamic-fields-callbacks-after-guard-b = Callback count after dropping guard_b: { $count }
example-dynamic-fields-after-update-5 = After updating the value to 5:

# --- hot_reload ---

example-hot-reload-started = === Hot reload example started ===
example-hot-reload-watched-file = Watched config file: { $path }
example-hot-reload-watcher-settings = WatcherConfig settings: debounce={ $debounce }ms, min_reload_interval={ $min_interval }ms, max_failures={ $max_failures }
example-hot-reload-port-not-found = "{ $port }" not found in template, cannot generate the demo modification
example-hot-reload-demo-write-completed = Demo write completed: changed { $old } to { $new } in the config file
example-hot-reload-demo-write-failed = Demo write failed: { $error }
example-hot-reload-waiting = Waiting for config file changes... (the example will modify the file in { $seconds }s)
example-hot-reload-ctrl-c-hint = Press Ctrl+C to exit early
example-hot-reload-exit-signal = Exit signal received, shutting down...
example-hot-reload-reload-skipped = Reload skipped: only { $elapsed }ms since last reload (min interval: { $min_interval }ms)
example-hot-reload-change-detected = Config file change detected: { $path }
example-hot-reload-reload-succeeded = Config reload succeeded!
example-hot-reload-reload-failed = Config reload failed: { $error } (consecutive failures: { $failures }/{ $max_failures })
example-hot-reload-failure-limit = Consecutive failure limit reached, pausing the watcher for { $pause }ms
example-hot-reload-watcher-stopped = File watcher stopped or failed, hot reload demo not completed
example-hot-reload-timeout = No config change event received within { $seconds }s, hot reload demo not completed.
example-hot-reload-no-fs-events = The current environment may not support filesystem event notifications (network filesystems, some container mounts, etc.).
example-hot-reload-demo-completed = === Hot reload demo completed, exiting ===
example-hot-reload-demo-not-completed = hot reload demo not completed
example-hot-reload-exiting = === Exiting ===
example-hot-reload-current-config = Current config (reloaded)
example-hot-reload-app-line = app: { $name } v{ $version } (environment: { $environment })
example-hot-reload-server-line = server: { $host }:{ $port } (max connections: { $max_connections })
example-hot-reload-logs-line = logs: level={ $level }, format={ $format }, output={ $output }
example-hot-reload-database-line = database: { $url } (max connections: { $max_connections }, timeout: { $timeout }s)
example-hot-reload-cache-line = cache: enabled={ $enabled }, TTL={ $ttl }s, max={ $max }MB

# --- config_groups ---

example-config-groups-started = Config groups example started
example-config-groups-title = Config Groups example
example-config-groups-current-env = Current environment: { $env }
example-config-groups-builder-header = ========== Load configuration with ConfigBuilder ==========
example-config-groups-chain-header = ========== Load configuration with SourceChainBuilder ==========
example-config-groups-environment = environment: { $env }
example-config-groups-source-chain = Configuration source chain:
example-config-groups-merged-sections = Merged top-level configuration sections: { $sections }
example-config-groups-priority-header = ========== Configuration priority demo ==========
example-config-groups-priority-result = Priority demo result:
example-config-groups-priority-name = name = { $name } (high priority overrides low priority)
example-config-groups-priority-port = port = { $port } (high priority overrides low priority)
example-config-groups-priority-level = level = { $level } (not set at high priority; low priority retained)
example-config-groups-final-header = ========== Final configuration ==========
example-config-groups-app-section = [App metadata]
example-config-groups-app-name = name: { $name }
example-config-groups-app-version = version: { $version }
example-config-groups-app-environment = environment: { $environment }
example-config-groups-server-section = [Server configuration]
example-config-groups-server-host = host: { $host }
example-config-groups-server-port = port: { $port }
example-config-groups-server-workers = workers: { $workers }
example-config-groups-database-section = [Database configuration]
example-config-groups-database-max-connections = Max connections: { $count }
example-config-groups-database-timeout = Connection timeout: { $seconds }s
example-config-groups-logging-section = [Logging configuration]
example-config-groups-logging-enabled = enabled: { $enabled }
example-config-groups-logging-level = level: { $level }
example-config-groups-logging-format = format: { $format }
example-config-groups-cache-section = [Cache configuration]
example-config-groups-cache-enabled = enabled: { $enabled }
example-config-groups-security-section = [Security configuration]
example-config-groups-security-origins = Allowed origins: { $origins }
example-config-groups-dev-section = [Development environment configuration]
example-config-groups-dev-hot-reload = Hot reload: { $enabled }
example-config-groups-dev-debug-endpoints = Debug endpoints: { $enabled }
example-config-groups-dev-verbose-errors = Verbose errors: { $enabled }
example-config-groups-prod-section = [Production environment configuration]
example-config-groups-prod-health-check = Health check: { $enabled }
example-config-groups-prod-rate-limit = Rate limit: { $enabled }
example-config-groups-prod-max-request-size = Max request size: { $bytes } bytes
example-config-groups-prod-monitoring = Performance monitoring: { $enabled }
example-config-groups-builder-loaded = ✓ ConfigBuilder loaded successfully!
example-config-groups-builder-failed = ✗ ConfigBuilder load failed: { $error }
example-config-groups-chain-loaded = ✓ SourceChainBuilder loaded successfully!
example-config-groups-chain-failed = ✗ SourceChainBuilder load failed: { $error }
example-config-groups-comparison-header = ========== Environment configuration comparison ==========
example-config-groups-dev-traits = dev env traits: localhost, port=3000, debug logs, cache disabled
example-config-groups-prod-traits = prod env traits: 0.0.0.0, port=8080, warn logs, cache enabled
example-config-groups-override-header = ========== Environment variable override demo ==========
example-config-groups-override-hint = You can override configuration with these environment variables:
example-config-groups-override-port = APP_SERVER_PORT=9000      - override server port
example-config-groups-override-database-url = APP_DATABASE_URL=...       - override database URL
example-config-groups-override-log-level = APP_LOGGING_LEVEL=debug   - override log level
example-config-groups-override-example = Example: APP_SERVER_PORT=9000 cargo run -p confers-examples --bin config_groups -- dev
example-config-groups-completed = Example completed!

# --- key_rotation ---

example-key-rotation-started = Key rotation example starting
example-key-rotation-all-completed = All example demos completed
example-key-rotation-example-1 = === Example 1: key version derivation ===
example-key-rotation-db-password-v1-key = Database password v1 key: { $key }
example-key-rotation-db-password-v2-key = Database password v2 key: { $key }
example-key-rotation-different-versions = Different versions produce different keys: validation passed
example-key-rotation-different-fields = Different fields produce different keys: validation passed
example-key-rotation-deterministic = The same version produces the same key (deterministic): validation passed
example-key-rotation-example-2 = === Example 2: encryption with versioned keys ===
example-key-rotation-v1-nonce = v1 encryption - nonce: { $nonce }
example-key-rotation-v1-ciphertext = v1 encryption - ciphertext: { $ciphertext }
example-key-rotation-v2-nonce = v2 encryption - nonce: { $nonce }
example-key-rotation-v2-ciphertext = v2 encryption - ciphertext: { $ciphertext }
example-key-rotation-different-ciphertexts = Different version keys produce different ciphertexts: validation passed
example-key-rotation-decrypt-correctly = Each version's key decrypts correctly: validation passed
example-key-rotation-cross-version-fail = Cross-version keys cannot decrypt: validation passed
example-key-rotation-example-3 = === Example 3: key rotation flow ===
example-key-rotation-old-version = Old version: { $version }
example-key-rotation-old-nonce = Old nonce: { $nonce }
example-key-rotation-old-ciphertext = Old ciphertext: { $ciphertext }
example-key-rotation-old-decrypted = Old key decryption succeeded: { $value }
example-key-rotation-new-version = New version: { $version }
example-key-rotation-new-nonce = New nonce: { $nonce }
example-key-rotation-new-ciphertext = New ciphertext: { $ciphertext }
example-key-rotation-rotated-data = Encrypted data after rotation: { $data }
example-key-rotation-rotation-completed = Key rotation completed
example-key-rotation-example-4 = === Example 4: encrypted data migration ===
example-key-rotation-stored-old-data = Stored old data: version={ $version }, nonce length={ $nonce_length }, ciphertext length={ $ciphertext_length }
example-key-rotation-migrated-data = Migrated data: version={ $version }, nonce length={ $nonce_length }, ciphertext length={ $ciphertext_length }
example-key-rotation-integrity-passed = Data integrity check: passed
example-key-rotation-old-key-cannot-decrypt = Old keys cannot decrypt new data: validation passed
example-key-rotation-example-5 = === Example 5: EnvKeyProvider and key versions ===
example-key-rotation-master-key-ok = Got the master key successfully
example-key-rotation-key-length = Key length: { $bytes } bytes
example-key-rotation-version-keys = Version { $version } - nonce: { $nonce }, ciphertext: { $ciphertext }
example-key-rotation-env-provider-ok = EnvKeyProvider integrates with multi-version keys: validation passed
example-key-rotation-get-key-failed = Failed to get the key: { $error }
example-key-rotation-example-6 = === Example 6: rollback mechanism ===
example-key-rotation-initial-v1 = Initial state: data encrypted with v1
example-key-rotation-after-rotation-v2 = After rotation: data encrypted with v2
example-key-rotation-stored-encrypted-data = Stored encrypted data: version={ $version }, previous_version={ $previous_version }
example-key-rotation-current-version-ok = ✓ current version (v2) decrypts successfully
example-key-rotation-historical-ok = ✓ decrypted historical data with previous_version successfully
example-key-rotation-rollback-complete = Rollback complete: data version reset to v1
example-key-rotation-current-previous = Current version: { $version }, previous_version: { $previous_version }
example-key-rotation-rollback-verified = ✓ data decryption verified after rollback
example-key-rotation-security-verified = ✓ v1 keys cannot decrypt data encrypted with v2 (security check passed)

# --- json_schema ---

example-json-schema-desc-application-name = Application name
example-json-schema-desc-server-port = Server port
example-json-schema-desc-enable-debug-mode = Enable debug mode
example-json-schema-desc-full-app-config = Full application configuration with server, database and cache
example-json-schema-desc-application-version = Application version
example-json-schema-desc-host-address = Host address
example-json-schema-desc-connection-timeout = Connection timeout (seconds)
example-json-schema-desc-database-host = Database host
example-json-schema-desc-database-port = Database port
example-json-schema-desc-database-name = Database name
example-json-schema-desc-max-connections = Max connections
example-json-schema-desc-caching-enabled = Whether caching is enabled
example-json-schema-desc-cache-ttl = Cache TTL (seconds)
example-json-schema-desc-cache-max-entries = Cache max entries
example-json-schema-desc-hostname = Hostname (RFC 1123 compliant)
example-json-schema-desc-port-number = Port number
example-json-schema-desc-admin-email = Administrator email
example-json-schema-desc-service-url = Service URL
example-json-schema-desc-rate-limit = Requests-per-second limit
example-json-schema-desc-cache-ttl-24h = Cache TTL (seconds, 0-24h)
example-json-schema-desc-config-tags = Configuration tags
example-json-schema-demo1 = === Demo 1: simple configuration Schema generation ===
example-json-schema-generated-schema = Generated JSON Schema:
example-json-schema-validate-valid = Validate a valid configuration: { $status }
example-json-schema-validate-invalid = Validate an invalid configuration: { $status }
example-json-schema-demo2 = === Demo 2: nested configuration Schema ===
example-json-schema-nested-schema = Generated nested JSON Schema (definitions section):
example-json-schema-validate-full = Validate the full configuration: { $status }
example-json-schema-demo3 = === Demo 3: Schema with validation constraints ===
example-json-schema-validated-schema = with validation constraints Schema:
example-json-schema-case-valid-config = valid config
example-json-schema-case-invalid-port = invalid port
example-json-schema-case-missing-required = missing required fields
example-json-schema-demo4 = === Demo 4: TypeScript type generation ===
example-json-schema-generated-typescript = Generated TypeScript type definitions:
example-json-schema-ide-integration = IDE integration:
example-json-schema-ide-copy-types = - Copy the TypeScript type definition file into the frontend project
example-json-schema-ide-quicktype = - Use JSON Schema generators (such as quicktype) to generate more types
example-json-schema-ide-vscode = - Use the schema file in VS Code for IntelliSense
example-json-schema-used-in-typescript = // Used in TypeScript:
example-json-schema-demo5 = === Demo 5: Schema export to file ===
example-json-schema-schema-exported = Schema exported to: { $path }
example-json-schema-verify-import-ok = verify import: success
example-json-schema-schema-title = Schema title: { $title }
example-json-schema-verify-import-failed = verify import: failed - { $error }
example-json-schema-export-failed = export failed: { $error }
example-json-schema-typescript-export-failed = TypeScript export failed: { $error }
example-json-schema-typescript-exported = TypeScript types exported to: { $path }
example-json-schema-demo6 = === Demo 6: ConfigSchema derive macro ===
example-json-schema-derive-intro = Generate the schema automatically with #[derive(ConfigSchema)]:
example-json-schema-auto-generated-methods = // auto-generated methods:
example-json-schema-export-schema-comment = // export schema:
example-json-schema-demo7 = === Demo 7: Schema use cases ===
example-json-schema-use-case-1 = 1. IDE IntelliSense
example-json-schema-use-case-1a = - Add the schema file to .vscode/settings.json
example-json-schema-use-case-1b = - JSON/YAML config files get autocompletion
example-json-schema-use-case-2 = 2. Config validation
example-json-schema-use-case-2a = - Validate config files against the schema at startup
example-json-schema-use-case-2b = - Validate configuration changes at the CI/CD stage
example-json-schema-use-case-3 = 3. API documentation
example-json-schema-use-case-3a = - auto-generate OpenAPI/Swagger specification
example-json-schema-use-case-3b = - Keep frontend and backend types in sync
example-json-schema-use-case-4 = 4. Multi-language code generation
example-json-schema-use-case-4a = - Use quicktype to generate Go/TypeScript/Python types from the schema
example-json-schema-use-case-4b = - Keep type definitions consistent across languages
example-json-schema-use-case-5 = 5. Configuration editor
example-json-schema-use-case-5a = - Render configuration forms in a web UI
example-json-schema-use-case-5b = - Generate validation rules from the schema
example-json-schema-cli-exported = Schema exported to: { $path }
example-json-schema-completed = Example completed

# --- validation ---

example-validation-title = Validation Example - config validation
example-validation-passed = Validation passed
example-validation-failed-count = Validation failed ({ $count } errors):
example-validation-pretty-ok = [OK] Configuration validation passed
example-validation-pretty-fail = [FAIL] Configuration validation failed
example-validation-error-field = - field: { $field }
example-validation-error-reason = reason: { $reason }
example-validation-total-errors = Total { $count } validation errors
example-validation-host-empty = host must not be empty
example-validation-host-too-long = host exceeds max length of 253 characters (current: { $length })
example-validation-host-invalid-chars = host contains invalid characters: { $host }
example-validation-port-reserved = port must not be 0 (reserved)
example-validation-port-privileged = port { $port } is a privileged port (< 1024), consider using an unprivileged port
example-validation-log-level-invalid = invalid log level: '{ $level }' (valid values: { $valid })
example-validation-email-empty = email must not be empty
example-validation-email-missing-at = invalid email format: missing @ symbol
example-validation-email-multiple-at = invalid email format: multiple @ symbols
example-validation-email-local-empty = invalid email format: local part is empty
example-validation-email-domain-empty = invalid email format: domain part is empty
example-validation-email-domain-dot = invalid email format: domain must contain a dot
example-validation-timeout-positive = timeout must be greater than 0
example-validation-timeout-too-long = timeout too long: { $timeout }s (max: 86400s = 24h)
example-validation-timeout-too-short = timeout too short: { $timeout }s (min: 1s)
example-validation-max-connections-zero = max connections must not be 0
example-validation-max-connections-too-many = too many max connections: { $connections } (recommended: <= 10000)
example-validation-url-empty = URL must not be empty
example-validation-url-scheme = URL must start with a valid scheme ({ $schemes })
example-validation-demo1 = === Demo 1: Basic field validation ===
example-validation-host-header = Host address validation:
example-validation-item-error = error: { $error }
example-validation-port-header = Port range validation:
example-validation-item-warning = warning: { $error }
example-validation-log-level-header = Log level validation:
example-validation-demo2 = === Demo 2: Format validation ===
example-validation-email-header = Email format validation:
example-validation-db-url-header = Database URL validation:
example-validation-demo3 = === Demo 3: Validation context ===
example-validation-default-context = Default context validation result:
example-validation-after-ignore = After ignoring 'port' and 'timeout_seconds':
example-validation-strict-mode = Strict mode (show details):
example-validation-demo4 = === Demo 4: Valid configuration validation ===
example-validation-validate-default = Validate default configuration:
example-validation-result-header = Validation result:
example-validation-demo5 = === Demo 5: Validation rule parsing ===
example-validation-unknown-rule = '{ $rule }' -> (unknown rule)
example-validation-demo6 = === Demo 6: Error handling strategies ===
example-validation-strategy-1 = Strategy 1: exit the program if validation fails
example-validation-detected-errors = Detected { $count } errors; the application cannot start
example-validation-fix-and-restart = Fix the configuration and restart
example-validation-strategy-2 = Strategy 2: use sentinel values to run with warnings
example-validation-demo7 = === Demo 7: Integration with Config Derive ===
example-validation-derive-intro = ConfigDerive validation integration:
example-validation-method-1 = Method 1: automatic validation with #[config(validate)]
example-validation-method-2 = Method 2: call validate() manually
example-validation-validation-failed-print = eprintln!("Validation failed: {"{"}:?{"}"}", report);
example-validation-method-3 = Method 3: custom validation with ValidationContext
example-validation-completed = Example completed
