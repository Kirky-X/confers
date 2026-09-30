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
    for (alias, canonical) in DEPRECATED_ALIASES {
        let var = format!("CARGO_FEATURE_{}", alias.to_uppercase().replace('-', "_"));
        if std::env::var_os(&var).is_some() {
            println!(
                "cargo:warning=confers: feature `{alias}` is deprecated and will be removed in the next release; use `{canonical}` instead"
            );
        }
    }
}
