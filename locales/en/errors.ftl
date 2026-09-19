# confers error catalog (en).
# One key per ConfigError variant; the English pattern mirrors the
# thiserror #[error] canonical Display string exactly (see the
# translate_en == Display guard test in src/error.rs).

error-file-not-found = Configuration file not found: { $filename }
error-parse-error = Failed to parse { $format }: { $message }
error-parse-error-at = Failed to parse { $format } at { $location }: { $message }
error-validation-failed = Validation failed for field '{ $field }': { $message } (rule: { $rule })
error-schema-validation-failed = schema validation failed with { $count } error(s)
error-decryption-failed = Decryption failed: { $message }
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
error-key = Encryption key error: { $message }
error-circular-reference = Circular reference detected: { $path }
error-lock-poisoned = Lock poisoned for resource '{ $resource }'
error-multi-source = Multiple sources failed
error-multi-source-detail = multiple sources failed: { $failed }/{ $total }
error-concurrency-conflict = Concurrency conflict on key '{ $key }': { $message }
error-key-rotation-failed = Key rotation failed from '{ $from_version }' to '{ $to_version }': { $reason }
error-watcher = Configuration watcher error: { $message }
error-override-blocked = Override blocked for key '{ $key }': { $reason }
error-health-check-failed = Health check failed: { $reason }
