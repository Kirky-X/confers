# confers error catalog (en).
# One key per ConfigError variant; the English pattern mirrors the
# thiserror #[error] canonical Display string exactly (see the
# translate_en == Display guard test in src/error.rs).
#
# Component message keys live here too: error-* fragments embedded into
# ConfigError.message / error_type fields (localized at the construction
# site), CryptoError variant keys, and log-* keys for operational log lines.

error-file-not-found = Configuration file not found: { $filename }
error-parse-error = Failed to parse { $format }: { $message }
error-parse-error-at = Failed to parse { $format } at { $location }: { $message }
error-validation-failed = Validation failed for field '{ $field }': { $message } (rule: { $rule })
error-schema-validation-failed = schema validation failed with { $count } error(s)
error-decryption-failed = Decryption failed.
error-remote-unavailable = Remote configuration source unavailable
error-version-mismatch = Configuration version mismatch: found { $found }, expected { $expected }
error-migration-failed = Migration failed from v{ $from } to v{ $to }: { $reason }
error-module-not-found = Module '{ $module }' not found in group '{ $group }'
error-reload-rolled-back = Configuration reload rolled back: { $reason }
error-io = IO error: { $message }
error-invalid-value = Invalid configuration value for '{ $key }': { $message }
error-source-chain = Source chain error: { $message }
error-timeout = Operation timed out after { $duration_ms }ms
error-size-limit-exceeded = Configuration size limit exceeded: { $actual } bytes (limit: { $limit })
error-interpolation = Interpolation error for '{ $variable }': { $message }
error-key = Encryption key error.
error-circular-reference = Circular reference detected: { $path }
error-lock-poisoned = Lock poisoned for resource '{ $resource }'
error-multi-source = Multiple sources failed
error-multi-source-detail = multiple sources failed: { $failed }/{ $total }
error-concurrency-conflict = Concurrency conflict on key '{ $key }': { $message }
error-key-rotation-failed = Key rotation failed from '{ $from_version }' to '{ $to_version }': { $reason }
error-watcher = Configuration watcher error: { $message }
error-override-blocked = Override blocked for key '{ $key }': { $reason }
error-health-check-failed = Health check failed: { $reason }

# --- component message fragments (embedded into ConfigError fields) ---

error-env-path-conflict = variable maps to config path '{ $path }', which collides with { $existing } at '{ $at }'; rename one of the conflicting variables
error-conflict-existing-nested-map = a nested map built from other variables
error-conflict-existing-scalar = a scalar value set by another variable
error-env-file-unreadable = Cannot read file referenced by { $var }
error-spawn-blocking-failed = spawn_blocking: { $message }

error-path-empty = file path must not be empty
error-path-cannot-resolve = Cannot resolve file path
error-path-access-denied = Access to { $path } is not allowed
error-path-credential-dir-denied = access to credential directories (.ssh, .aws, .gnupg, .kube, .gcloud, .env) is not allowed
error-path-not-regular-file = Only regular files can be read
error-path-extension-denied = File extension { $ext } is not allowed

error-decrypt-key-derivation-failed = key derivation failed
error-decrypt-payload-not-base64 = payload is not valid base64
error-decrypt-payload-too-short = payload too short for a nonce
error-decrypt-open-failed = decryption failed
error-decrypt-plaintext-not-utf8 = plaintext is not valid UTF-8
error-decrypt-malformed-envelope = malformed envelope
error-decrypt-master-key-missing = master key missing

error-key-plaintext-not-utf8 = Invalid plaintext UTF-8: { $message }
error-key-store-temp-open-failed = Failed to open temp key store: { $message }
error-key-rotation-verification-failed = master key rotation rejected: old master key verification failed
error-key-file-weak-material = Key file contains weak key material (constant-byte); generate a random key instead

error-crypto-encryption-failed = encryption failed
error-crypto-decryption-failed = decryption failed
error-crypto-invalid-key-length = invalid key length: expected exactly 32 bytes for XChaCha20-Poly1305, got { $actual } bytes
error-crypto-weak-key = weak key rejected (constant-byte key material such as all-zero keys is not secure)
error-crypto-key-not-found = key not found
error-crypto-legacy-decryption-failed = legacy decryption failed (AES-256-GCM)

error-redis-pubsub-connect-timeout = redis_pubsub: connect timeout (10s)
error-redis-pubsub-connect-failed = redis_pubsub: { $message }
error-redis-subscribe-failed = redis_subscribe: { $message }
error-vault-token-not-provided = Vault token not provided
error-vault-request-failed = vault_request: { $message }

error-k8s-ca-read-failed = Failed to read Kubernetes CA file '{ $path }': { $message }
error-k8s-ca-no-pem-block = Invalid Kubernetes CA certificate in '{ $path }': no PEM certificate block found
error-k8s-ca-invalid = Invalid Kubernetes CA certificate in '{ $path }': { $message }
error-k8s-client-build-failed = failed to build k8s HTTP client: { $message }
error-k8s-circuit-breaker-lock-poisoned = k8s circuit breaker lock poisoned

error-nacos-auth-password-missing = nacos auth requires both username and password; password is missing
error-nacos-auth-username-missing = nacos auth requires both username and password; username is missing
error-nacos-login-request-failed = nacos login request failed
error-nacos-login-rejected = nacos login rejected with status { $status }
error-nacos-login-body-unreadable = nacos login returned an unreadable body
error-nacos-login-no-token = nacos login response has no accessToken
error-nacos-request-failed = nacos request failed: { $message }
error-nacos-unauthorized = nacos returned 401 Unauthorized (accessToken rejected)
error-nacos-circuit-breaker-open = nacos source circuit breaker is open

# --- operational log lines ---

log-redis-pubsub-connection-lost = Redis config-bus pubsub connection lost (channel '{ $channel }'); reconnecting in { $backoff }
log-redis-pubsub-reconnect-failed = Redis config-bus pubsub reconnect failed (channel '{ $channel }'): { $message }; retrying in { $backoff }
log-fs-watch-failed = failed to watch { $path }: { $message }
log-fs-watcher-parent-removed = file watcher: watched parent directory { $path } removed; the kernel watch is gone and cannot be re-armed
log-fs-watcher-channel-full = file watcher event channel full; dropped change event for { $path } (total dropped: { $total })
log-reload-validation-commit-anyway = reload validation failed but rollback_on_validation_failure is disabled; committing anyway: { $reason }
log-post-commit-migration-failed = post-commit migration { $from }→{ $to } failed (configuration stays committed): { $message }
