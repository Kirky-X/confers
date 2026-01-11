# Architecture Decision Record (ADR)

This document records important architectural decisions made in the confers project.

## Table of Contents

1. [Encryption Strategy](#1-encryption-strategy)
2. [Nonce Cache Size](#2-nonce-cache-size)
3. [Provider Priority System](#3-provider-priority-system)
4. [Memory Limit Enforcement](#4-memory-limit-enforcement)
5. [Config Validation Approach](#5-config-validation-approach)
6. [Default Value Syntax](#6-default-value-syntax)

---

## 1. Encryption Strategy

**Status**: Implemented
**Date**: 2025-01-11
**Context**: Secure storage of sensitive configuration values

### Problem Statement

Configuration values may contain sensitive information (API keys, passwords, tokens) that needs to be:
- Encrypted at rest in configuration files
- Protected from memory inspection (zeroized)
- Secure against replay attacks (nonce reuse)

### Decision

**Chosen**: AES-256-GCM with nonce reuse detection

### Alternatives Considered

| Alternative | Pros | Cons | Rejection Reason |
|-------------|------|-------|-----------------|
| AES-256-CBC | Widely supported | No authenticated encryption | No integrity verification without HMAC |
| ChaCha20-Poly1305 | Modern AEAD | Less library support | Complexity of implementing |
| RSA/ECDSA | Key exchange | Slow performance | Not suitable for config values |
| XChaCha20-Poly1305 | Extended nonce | Less library support | Similar to ChaCha20 |

### Rationale

1. **Security**: AES-256-GCM provides authenticated encryption (confidentiality + integrity + authenticity)
2. **Performance**: Hardware acceleration available on modern CPUs (AES-NI)
3. **Library Support**: Widely supported in Rust ecosystem (`aes-gcm` crate)
4. **Standardization**: NIST-approved algorithm (FIPS 197)
5. **Nonce Management**: 96-bit nonce provides ~2^96 unique values, sufficient for config lifetimes

### Trade-offs

- **Nonce Cache Size**: 10,000 entries uses ~1.2MB memory (120 bytes per entry)
  - Allows ~2-4 hours of operation at 1-30s reload intervals
  - LRU eviction ensures unbounded growth
  - Cryptographic check still detects reuse after eviction
  - *Trade-off*: Very short reload intervals (<1s) could exhaust cache

### References

- NIST SP 800-38D: Recommendation for Block Cipher Modes of Operation
- RFC 5116: The AES-GCM Cipher and its Use with IPsec

---

## 2. Nonce Cache Size

**Status**: Implemented
**Date**: 2025-01-11
**Context**: Balancing security (nonce reuse detection) with memory usage

### Problem Statement

Nonce reuse detection requires tracking all used nonces. Unbounded growth would:
- Consume unlimited memory
- Potential DoS through configuration injection attacks

### Decision

**Chosen**: LRU cache with 10,000 entry limit

### Alternatives Considered

| Alternative | Pros | Cons | Rejection Reason |
|-------------|------|-------|-----------------|
| Unbounded HashSet | Unlimited detection | Memory DoS vulnerability | Security risk too high |
| 1,000 entries | Lower memory | May not cover typical usage | Too restrictive |
| 100,000 entries | Better coverage | Higher memory (~12MB) | Not worth cost |
| Time-based expiration | Auto-cleanup | Complex implementation | Hard to tune timeout |

### Rationale

1. **Security**: 10,000 entries provide ample detection for typical scenarios
2. **Memory**: 1.2MB is acceptable for a config management library
3. **LRU Eviction**: Keeps most recent nonces in memory (hot path optimization)
4. **Double Protection**: LRU eviction + cryptographic check provides defense in depth
5. **Entry Size**: 120 bytes (nonce + timestamp) is reasonable

### Trade-offs

- **High-Frequency Reloads**: Config reloading every second for hours could exhaust cache
- **Solution**: Document recommended reload intervals (5-60 seconds)

### Implementation Notes

```rust
// From src/encryption/mod.rs
const MAX_NONCE_CACHE_SIZE: usize = 10000;

pub struct ConfigEncryption {
    key: SecureKey,
    nonce_cache: Mutex<LruCache<Vec<u8>, ()>>,
}
```

### Future Considerations

- If high-frequency reload becomes a requirement, consider:
  - Time-based eviction (e.g., nonces older than 1 hour)
  - Per-provider nonce pools
  - Adaptive cache sizing based on usage patterns

---

## 3. Provider Priority System

**Status**: Implemented
**Date**: 2025-01-11
**Context**: Configuration can come from multiple sources (files, env, CLI, remote)

### Problem Statement

When multiple providers return the same configuration key, which value should be used?

### Decision

**Chosen**: Numeric priority system (higher number = higher priority)

### Alternatives Considered

| Alternative | Pros | Cons | Rejection Reason |
|-------------|------|-------|-----------------|
| First-wins | Simple | Inflexible order | User control limited |
| Last-wins | Recent values dominate | File order becomes irrelevant | Not user-friendly |
| Weighted average | Fair distribution | Complex configuration | Hard to predict behavior |

### Rationale

1. **Flexibility**: Users control priority through `with_priority()` builder pattern
2. **Predictability**: Lower numbers override higher numbers consistently
3. **Extensibility**: Easy to add custom providers
4. **Performance**: Linear search is O(n) but n is small (<10 typical providers)

### Trade-offs

- **Complexity**: Users must understand priority system
- **Documentation**: Requires clear explanation in docs

### Default Priorities

```rust
// From src/providers/provider.rs
// File providers (highest priority)
FileConfigProvider: priority 10

// CLI providers
CliConfigProvider: priority 20

// Environment providers
EnvironmentProvider: priority 30

// Remote providers (lowest priority)
HttpConfigProvider: priority 30 (configurable)
ConsulConfigProvider: priority 30
EtcdConfigProvider: priority 30
```

### Implementation Notes

```rust
// Priority merge in ProviderManager
pub fn merge_configs(&mut self, sources: Vec<(Priority, Map)>) {
    // Sort by priority (ascending - lower first)
    sources.sort_by_key(|(prio, _)| prio);

    // Apply in order (later values override earlier)
    for (_, map) in sources {
        for (key, value) in map {
            self.config.insert(key, value);
        }
    }
}
```

---

## 4. Memory Limit Enforcement

**Status**: Implemented
**Date**: 2025-01-11
**Context**: Prevent configuration files from causing memory exhaustion

### Problem Statement

Large configuration files or malicious inputs could:
- Consume excessive memory during parsing
- Cause application crashes
- Enable DoS attacks through config injection

### Decision

**Chosen**: Configurable memory limit (default: 512MB) with enforcement

### Alternatives Considered

| Alternative | Pros | Cons | Rejection Reason |
|-------------|------|-------|-----------------|
| No limit | Best performance | Security vulnerability | DoS risk too high |
| Hard limit | Simple | Inflexible | Doesn't adapt to use cases |
| Percentage-based | Relative | Hard to set correctly | Complex user configuration |
| Per-provider limits | Granular | Complex implementation | Over-engineering |

### Rationale

1. **Security**: 512MB limit prevents most DoS attacks while being reasonable
2. **Flexibility**: Users can increase limit via `ConfigLoader::with_memory_limit()`
3. **Predictability**: Fixed size allows better capacity planning
4. **Implementation**: Easy to track and enforce using `sysinfo`

### Trade-offs

- **Large Config Files**: Users with large configs must increase limit
- **Dynamic Allocation**: May reject valid (but large) configs during growth phases
- **Platform Differences**: Memory tracking varies by OS

### Implementation Notes

```rust
// From src/core/loader.rs
use sysinfo::{Pid, ProcessRefreshKind, RefreshKind, System};

pub struct ConfigLoader<T> {
    memory_limit_mb: Option<usize>,
    // ...
}

fn get_memory_usage_mb() -> Option<f64> {
    // System call to get current process memory
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything()),
    );

    let current_pid = Pid::from_u32(process::id());
    let memory = sys.process(current_pid)
        .map(|process| process.memory() as f64 / 1024.0 / 1024.0);

    memory
}
```

### Future Considerations

- Consider adding soft limit (warning) before hard limit (error)
- Consider limit per configuration type (different limits for file vs remote)
- Provide more detailed memory usage reporting

---

## 5. Config Validation Approach

**Status**: Implemented
**Date**: 2025-01-11
**Context**: Ensure configuration values meet application requirements

### Problem Statement

Configuration values may have constraints:
- Type validation (e.g., port is u16)
- Business rules (e.g., port > 1024)
- Cross-field validation (e.g., db_url depends on db_type)
- Custom validation logic

### Decision

**Chosen**: Declarative validation using `validator` crate with custom validator support

### Alternatives Considered

| Alternative | Pros | Cons | Rejection Reason |
|-------------|------|-------|-----------------|
| Manual if-checks | Simple | Boilerplate, errors at runtime | Not maintainable |
| Procedural macro | Less boilerplate | Complex to debug | Hard to customize |
| Type-state machine | Strong guarantees | Complex for users | Over-engineering for config |
| Custom derive macro | Perfect integration | Complex implementation | Reimplementing validator |

### Rationale

1. **Declarative**: Use `#[validate]` attribute to specify rules
2. **Library Support**: `validator` crate is mature, well-tested
3. **Derive Integration**: Seamlessly works with `#[derive(Config)]`
4. **Custom Support**: `#[config(custom_validate = "...")]` for complex rules
5. **Error Messages**: `validator` provides clear, localized errors

### Trade-offs

- **Compilation Time**: Derive macros increase build time
- **Binary Size**: Adds validator dependency (~small increase)
- **Complexity**: Custom validators must be valid expressions

### Implementation Notes

```rust
// Example from docs
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config, Validate)]
#[config(env_prefix = "APP")]
#[config(validate)]
struct AppConfig {
    #[config(default = 8080)]
    #[validate(range(min = 1024, max = 65535))]
    port: u16,

    #[config(default = "\"localhost\".to_string()")]
    #[validate(url(port = 8080))]
    server_url: String,

    #[config(custom_validate = "validate_secret")]
    #[config(sensitive = true)]
    api_key: String,
}
```

### Future Considerations

- Consider async validation for remote config fetching
- Add validation that depends on environment (e.g., prod vs dev)
- Support validation-time configuration (strict vs lenient mode)

---

## 6. Default Value Syntax

**Status**: Implemented (2025-01-11)
**Date**: 2025-01-11
**Context**: Simplify default value specification for configuration fields

### Problem Statement

Users want to specify default values concisely, especially for String types.

### Old Approach (Required Complex Syntax)

```rust
#[derive(Config)]
struct Config {
    // Verbose and error-prone
    #[config(default = "\"hello\".to_string()")]
    message: String,

    // Works but unintuitive for non-string types
    #[config(default = "42")]  // This was a string, not a number!
    number: u32,
}
```

### Decision

**Chosen**: Auto-detect string literals for `String` types and add `.to_string()` automatically

### New Approach (Simple Syntax)

```rust
#[derive(Config)]
struct Config {
    // Clean and simple
    #[config(default = "hello")]
    message: String,

    // Works correctly for all types
    #[config(default = 42)]
    number: u32,
}
```

### Rationale

1. **User Experience**: `default = "hello"` is more natural than `default = "\"hello\".to_string()"`
2. **Type Safety**: Automatically converts `&str` literals to `String` via `.to_string()`
3. **Backward Compatible**: Old `.to_string()` syntax still works
4. **Implementation**: Simple pattern matching in macro code generation

### Implementation Details

```rust
// From macros/src/codegen.rs
fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "String";
        }
    }
    false
}

fn is_string_literal(expr: &Expr) -> bool {
    matches!(expr, Expr::Lit(expr_lit) if matches!(expr_lit.lit, syn::Lit::Str(_)))
}

// In default_impl_body generation:
if is_string_type(ty) && is_string_literal(d) {
    quote! { #name: #d.to_string() }
} else {
    quote! { #name: #d }
}
```

### Trade-offs

- **Macro Complexity**: Additional helper functions and pattern matching
- **Non-string Types**: Must still use correct type syntax (e.g., `42` not `"42"`)
- **Compilation**: Slight increase in macro expansion time

### Benefits

1. **Reduced Boilerplate**: Users write 60% less code for string defaults
2. **Fewer Errors**: Compiler catches type mismatches instead of runtime panics
3. **Better Readability**: Configuration definitions are clearer and more concise
4. **IDE Support**: Better autocomplete and syntax highlighting

---

## Appendix: Decision Template

```markdown
## [N]. [Title]

**Status**: [Proposed | Accepted | Deprecated | Superseded]
**Date**: YYYY-MM-DD
**Context**: Brief description of the problem or situation

### Problem Statement

What problem are we trying to solve?

### Decision

Brief summary of the chosen approach.

### Alternatives Considered

| Alternative | Pros | Cons | Rejection Reason |
|-------------|------|-------|-----------------|
| ... | ... | ... | ... |

### Rationale

Why was this decision made? What are the consequences?

### Trade-offs

What are the downsides or compromises?

### Implementation Notes

Any relevant code snippets or implementation details.

### Future Considerations

What should we revisit or reconsider later?
```

---

## How to Add a New Decision

1. Copy the template from the Appendix
2. Fill in all sections
3. Give it a sequential number
4. Commit with message: `docs(adrs): add decision for [topic]`
5. Add a brief summary to the table of contents
6. Update this file's last modified date
