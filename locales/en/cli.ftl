# confers CLI catalog (en).

cli-about = Configuration diagnostics tool for confers
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
