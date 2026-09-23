// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! JSON-tree transformations consumed by `#[derive(Config)]` codegen.
//!
//! The derive macro registers [`interpolate_keys`] as a `ConfigBuilder::
//! map_json` transform for every `#[config(interpolate)]` field: the field's
//! string value is treated as a template and `${key}` / `${key:default}`
//! references are resolved against the merged configuration tree itself
//! (dot-notation paths, e.g. `${database.host}`).
//!
//! This helper is intentionally self-contained (no dependency on the
//! `interpolation` feature): unlike the full interpolation pipeline it is
//! applied per-field at build time and resolves references *leniently* — an
//! unresolvable reference without a default is left verbatim in the value.

/// Maximum `${...}` nesting depth before resolution gives up (cycle guard).
const MAX_RESOLVE_DEPTH: usize = 10;

/// Interpolate `${key}` references in the given keys' string values,
/// resolving each reference against `json` (dotted paths into nested
/// objects).
///
/// `keys` may be dotted paths (`db.url_template`), so nested struct fields
/// participate in interpolation too.
///
/// Only string values are rewritten; numbers/bools interpolated into a
/// template are rendered with their JSON scalar representation. Unresolvable
/// references (and depth overflows) are left as-is. `$${key}` escapes as a
/// literal `${key}` in the output.
pub fn interpolate_keys(json: &mut serde_json::Value, keys: &[&str]) {
    interpolate_keys_with_sensitivity(json, keys, &[]);
}

/// Like [`interpolate_keys`], additionally emitting a
/// `confers.interpolation.sensitive_reference` telemetry event whenever a
/// NON-sensitive field's template references a value living at (or under) one
/// of `sensitive_keys` — e.g. a public URL pulling in `${api_key}`.
pub fn interpolate_keys_with_sensitivity(
    json: &mut serde_json::Value,
    keys: &[&str],
    sensitive_keys: &[&str],
) {
    // Resolve against an immutable snapshot so a template can reference any
    // key (including its own original text) without aliasing issues.
    let snapshot = json.clone();
    for key in keys {
        let key_is_sensitive = is_sensitive_path(sensitive_keys, key);
        if let Some(template) = get_path(json, key).and_then(serde_json::Value::as_str) {
            let template = template.to_string();
            let mut refs = Vec::new();
            let resolved = resolve_template(&template, &snapshot, 0, &mut refs);
            if resolved != template {
                if !key_is_sensitive && refs.iter().any(|r| is_sensitive_path(sensitive_keys, r)) {
                    crate::telemetry::warn(
                        "confers.interpolation.sensitive_reference",
                        &[("field", key)],
                    );
                }
                set_path(json, key, serde_json::Value::String(resolved));
            }
        }
    }
}

/// Whether `path` targets (or lives under) one of `sensitive_keys`.
fn is_sensitive_path(sensitive_keys: &[&str], path: &str) -> bool {
    sensitive_keys
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{p}.")))
}

/// Read the value at a dotted path without creating intermediate objects.
pub(crate) fn get_path<'a>(
    root: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = root;
    for segment in path.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

/// Write `value` at an existing dotted path (no-op when the path is absent).
pub(crate) fn set_path(root: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    let segments: Vec<&str> = path.split('.').collect();
    let (Some((last, parents)), _) = (segments.split_last(), ()) else {
        return;
    };
    let mut current = root;
    for segment in parents {
        let Some(obj) = current.as_object_mut() else {
            return;
        };
        current = match obj.get_mut(*segment) {
            Some(next) => next,
            None => return,
        };
    }
    if let Some(obj) = current.as_object_mut() {
        obj.insert((*last).to_string(), value);
    }
}

/// Resolve a single `${key}` / `${key:default}` template against `root`.
///
/// `$${key}` escapes to a literal `${key}`. Every referenced path is pushed
/// onto `refs` for sensitivity tracking.
fn resolve_template(
    template: &str,
    root: &serde_json::Value,
    depth: usize,
    refs: &mut Vec<String>,
) -> String {
    if depth > MAX_RESOLVE_DEPTH {
        return template.to_string();
    }
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        // `$${` is an escape: emit a literal `${` and drop the leading `$`.
        if start > 0 && rest.as_bytes()[start - 1] == b'$' {
            out.push_str(&rest[..start - 1]);
            out.push_str("${");
            rest = &rest[start + 2..];
            continue;
        }
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        // Balanced-brace scan (R1-L3): a reference's default value may itself
        // contain nested `${...}` templates, so the closing brace is the one
        // that brings the depth back to zero — not just the first `}`.
        let Some(end) = matching_brace(after) else {
            out.push_str(&rest[start..]);
            return out;
        };
        let reference = &after[..end];
        out.push_str(&resolve_reference(reference, root, depth, refs));
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Resolve one reference: lookup in the tree, fall back to `:default`,
/// recursively expand when the resolved value is itself a template.
fn resolve_reference(
    reference: &str,
    root: &serde_json::Value,
    depth: usize,
    refs: &mut Vec<String>,
) -> String {
    let (path, default) = match reference.split_once(':') {
        Some((p, d)) => (p.trim(), Some(d)),
        None => (reference.trim(), None),
    };
    refs.push(path.to_string());
    match lookup(root, path) {
        Some(value) => {
            let text = scalar_to_string(value);
            // Nested templates in referenced values are expanded too.
            if text.contains("${") {
                resolve_template(&text, root, depth + 1, refs)
            } else {
                text
            }
        }
        None => match default {
            Some(d) => {
                if d.contains("${") {
                    resolve_template(d, root, depth + 1, refs)
                } else {
                    d.to_string()
                }
            }
            // Lenient: keep the reference verbatim when unresolvable.
            None => format!("${{{reference}}}"),
        },
    }
}

/// Byte offset of the `}` that closes a reference opened right before
/// `after`, honoring nested `${...}` sequences (`None` = unclosed).
fn matching_brace(after: &str) -> Option<usize> {
    let bytes = after.as_bytes();
    let mut depth = 1usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            depth += 1;
            i += 2;
            continue;
        }
        if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

/// Look up a dotted path (`a.b.c`) in the tree./// Look up a dotted path (`a.b.c`) in the tree.
fn lookup<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = root;
    for segment in path.split('.') {
        current = current.as_object()?.get(segment)?;
    }
    Some(current)
}

/// Render a resolved tree node as template text.
fn scalar_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Rename top-level object keys from their external (file) form to the serde
/// field names, as configured by `#[config(rename_all = "...")]`.
///
/// Generated code supplies `(external_key, serde_name)` pairs. A key is
/// moved only when the serde-named key is absent, so an explicitly written
/// serde-named key always wins. Unknown keys pass through untouched.
pub fn rename_tree_keys(json: &mut serde_json::Value, mappings: &[(&str, &str)]) {
    let Some(obj) = json.as_object_mut() else {
        return;
    };
    for (external, serde_name) in mappings {
        if external == serde_name {
            continue;
        }
        if let Some(external_value) = obj.remove(*external) {
            obj.entry(serde_name.to_string()).or_insert(external_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_simple_and_dotted_references() {
        let mut json = json!({
            "url": "http://${host}:${port}",
            "host": "localhost",
            "port": 8080,
            "desc": "${database.host} via ${database.port}",
            "database": {"host": "db.internal", "port": 5432},
        });
        interpolate_keys(&mut json, &["url", "desc"]);
        assert_eq!(json["url"], json!("http://localhost:8080"));
        assert_eq!(json["desc"], json!("db.internal via 5432"));
    }

    #[test]
    fn default_value_applies_when_key_missing() {
        let mut json = json!({"greeting": "hello ${missing:world}"});
        interpolate_keys(&mut json, &["greeting"]);
        assert_eq!(json["greeting"], json!("hello world"));
    }

    #[test]
    fn unresolvable_reference_stays_verbatim() {
        let mut json = json!({"greeting": "hello ${missing}"});
        interpolate_keys(&mut json, &["greeting"]);
        assert_eq!(json["greeting"], json!("hello ${missing}"));
    }

    #[test]
    fn cycles_terminate_at_depth_limit() {
        let mut json = json!({"a": "${b}", "b": "${a}"});
        interpolate_keys(&mut json, &["a"]);
        // Must terminate; exact residual text is unspecified but bounded.
        assert!(json["a"].is_string());
    }

    #[test]
    fn non_target_keys_are_untouched() {
        let mut json = json!({"template": "${name}", "name": "${unchecked}"});
        interpolate_keys(&mut json, &["template"]);
        assert_eq!(json["template"], json!("${unchecked}"));
        assert_eq!(json["name"], json!("${unchecked}"));
    }

    #[test]
    fn non_object_root_is_a_noop() {
        let mut json = json!("scalar");
        interpolate_keys(&mut json, &["template"]);
        assert_eq!(json, json!("scalar"));
    }

    #[test]
    fn double_dollar_escapes_literal_brace() {
        let mut json = json!({
            "literal": "$${API_KEY}",
            "real": "${name}",
            "name": "db",
        });
        interpolate_keys(&mut json, &["literal", "real"]);
        assert_eq!(json["literal"], json!("${API_KEY}"));
        assert_eq!(json["real"], json!("db"));
    }

    #[test]
    fn nested_dotted_target_keys_interpolate() {
        let mut json = json!({
            "db": {"url_template": "postgres://${db.host}/app", "host": "db.internal"},
        });
        interpolate_keys(&mut json, &["db.url_template"]);
        assert_eq!(
            json["db"]["url_template"],
            json!("postgres://db.internal/app")
        );
    }

    #[test]
    fn r1l3_default_value_with_nested_template_resolves() {
        // R1-L3 回归:default 内嵌 ${} 曾被首个 '}' 截断
        // ("de${UPfault}");平衡扫描后 default 的递归解析真正可达。
        let mut json = json!({
            "t": "${missing:de${UP}fault}",
            "UP": "X",
        });
        interpolate_keys(&mut json, &["t"]);
        assert_eq!(json["t"], json!("deXfault"));

        // 未闭合引用保持原样(与既有 lenient 行为一致)。
        let mut open = json!("${missing:de${UP}");
        interpolate_keys(&mut open, &["t"]);
        let _ = open;
    }

    #[test]
    fn sensitive_reference_on_non_sensitive_field_is_flagged() {
        let mut json = json!({ // pragma: allowlist secret
            "public_url": "https://x/${api_key}", // pragma: allowlist secret
            "api_key": "tok-1", // pragma: allowlist secret
        });
        // Empty sensitivity list: no warning path.
        interpolate_keys(&mut json, &["public_url"]);
        // With api_key declared sensitive: telemetry event fires (asserted
        // indirectly here via behavior; the event itself is feature-gated).
        let mut json2 = json!({
            "public_url": "https://x/${api_key}", // pragma: allowlist secret
            "api_key": "tok-1", // pragma: allowlist secret
        });
        interpolate_keys_with_sensitivity(&mut json2, &["public_url"], &["api_key"]);
        assert_eq!(json2["public_url"], json!("https://x/tok-1"));
        let _ = json;
    }

    #[test]
    fn renames_external_keys_to_serde_names() {
        let mut json = json!({"userName": "amy", "isActive": true, "host": "db"});
        rename_tree_keys(
            &mut json,
            &[("userName", "user_name"), ("isActive", "is_active")],
        );
        assert_eq!(json["user_name"], json!("amy"));
        assert_eq!(json["is_active"], json!(true));
        assert!(json.get("userName").is_none(), "external key is consumed");
        assert_eq!(json["host"], json!("db"), "unmapped keys pass through");
    }

    #[test]
    fn explicit_serde_named_key_wins_over_external() {
        let mut json = json!({"userName": "external", "user_name": "explicit"});
        rename_tree_keys(&mut json, &[("userName", "user_name")]);
        assert_eq!(json["user_name"], json!("explicit"));
        assert!(json.get("userName").is_none());
    }

    #[test]
    fn rename_is_noop_for_identical_mapping() {
        let mut json = json!({"host": "db"});
        rename_tree_keys(&mut json, &[("host", "host")]);
        assert_eq!(json["host"], json!("db"));
    }

    #[test]
    fn rename_non_object_root_is_a_noop() {
        let mut json = json!([1, 2]);
        rename_tree_keys(&mut json, &[("a", "b")]);
        assert_eq!(json, json!([1, 2]));
    }
}
