//! Copyright (c) 2026 Kirky.X🌠
//! SPDX-License-Identifier: MIT

//! Deprecated-feature notices.
//!
//! Cargo has no native feature deprecation, so renamed features survive one
//! release as alias features pointing at the canonical name; this script
//! turns each enabled alias into a compile-time `cargo:warning` so callers
//! see the migration before the alias disappears.
//!
//! Current aliases (see `[features]`): `env` → `dotenv`,
//! `key` → `key-management`, `schema` → `json-schema`.

const DEPRECATED_ALIASES: &[(&str, &str)] = &[
    ("env", "dotenv"),
    ("key", "key-management"),
    ("schema", "json-schema"),
];

fn main() {
    // Default is "rerun on any workspace change"; narrowing to this script
    // keeps alias deprecation notices from triggering spurious rebuilds.
    println!("cargo:rerun-if-changed=build.rs");
    // security 与 encryption 已解耦：仅启用 security 不再隐含 encryption，
    // 依赖 #[config(encrypt = ...)] 字段加密的场景若漏开 encryption，解密会
    // 静默失效——编译期提示一次，把误配提前到构建可见。
    if std::env::var_os("CARGO_FEATURE_SECURITY").is_some()
        && std::env::var_os("CARGO_FEATURE_ENCRYPTION").is_none()
    {
        println!(
            "cargo:warning=confers: feature `security` no longer implies `encryption`; if you rely on `#[config(encrypt = ...)]` field encryption, enable `encryption` explicitly or decryption will silently fail"
        );
    }
    for (alias, canonical) in DEPRECATED_ALIASES {
        let var = format!("CARGO_FEATURE_{}", alias.to_uppercase().replace('-', "_"));
        if std::env::var_os(&var).is_some() {
            println!(
                "cargo:warning=confers: feature `{alias}` is deprecated and will be removed in the next release; use `{canonical}` instead"
            );
        }
    }
}
