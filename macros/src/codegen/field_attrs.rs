// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Codegen for opt-in field capabilities: `#[config(dynamic)]` and
//! `#[config(watch)]`.
//!
//! - `dynamic` fields get a `<field>_handle()` method returning a
//!   [`confers::DynamicField`] seeded with the loaded value: the handle can
//!   be pushed updates at runtime and read/subscribed without reloading the
//!   whole struct.
//! - `watch` fields are subscribed by a generated `field_watcher`
//!   constructor wrapping [`confers::watcher::StructFieldWatcher`] (requires
//!   the `watch` feature).
//!
//! Every derived struct also receives a `ConfigFieldKeys` implementation
//! exposing its serde field names — the hook `#[config(flatten)]` codegen
//! uses to hoist top-level keys into a flattened nested struct.
//!
//! Note: the struct-level `#[config(watch)]` attribute keeps its documented
//! "enable file watching" meaning and deliberately does not generate the
//! field watcher; only the explicit field-level attribute opts in, so
//! existing structs never gain new methods referencing feature-gated APIs.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::parse::FieldAttrs;

/// Generate the `ConfigFieldKeys` impl for a derived struct.
///
/// `FIELD_KEYS` lists the serde field names (the keys the loader writes into
/// the merged JSON tree), in declaration order. Used by `flatten` codegen to
/// know which top-level keys belong to a flattened nested struct.
pub fn generate_field_keys_impl(struct_ident: &Ident, fields: &[(&Ident, &syn::Type, FieldAttrs)]) -> TokenStream {
    let keys: Vec<TokenStream> = fields
        .iter()
        .map(|(ident, _, _)| {
            let key = ident.to_string();
            quote! { #key }
        })
        .collect();

    quote! {
        impl confers::ConfigFieldKeys for #struct_ident {
            const FIELD_KEYS: &'static [&'static str] = &[#(#keys),*];
        }
    }
}

/// Generate the `dynamic`/`watch` companion impl for a derived struct.
///
/// Returns an empty stream when neither attribute is used anywhere, keeping
/// generated code unchanged (and dependency-free) for structs that do not
/// opt in.
pub fn generate_field_attr_impls(
    struct_ident: &Ident,
    fields: &[(&Ident, &syn::Type, FieldAttrs)],
) -> TokenStream {
    let dynamic_methods = fields
        .iter()
        .filter(|(_, _, f)| f.dynamic)
        .map(|(ident, ty, _)| {
            let method = quote::format_ident!("{}_handle", ident);
            quote! {
                /// Runtime handle for this `#[config(dynamic)]` field.
                ///
                /// Seeded with the loaded value; push fresh values after a
                /// reload with [`confers::DynamicField::update`] and read the
                /// latest value lock-free via `get()`.
                pub fn #method(&self) -> confers::DynamicField<#ty> {
                    confers::DynamicField::new(self.#ident.clone())
                }
            }
        });

    let watch_fields: Vec<&(&Ident, &syn::Type, FieldAttrs)> =
        fields.iter().filter(|(_, _, f)| f.watch).collect();

    let watcher_method = if watch_fields.is_empty() {
        quote! {}
    } else {
        let extractors = watch_fields.iter().map(|(ident, _, _)| {
            let name = ident.to_string();
            quote! {
                (
                    ::std::convert::From::from(#name),
                    ::std::boxed::Box::new(|c: &Self| {
                        confers::ConfigValue::from_serializable(&c.#ident)
                    }) as confers::watcher::FieldExtractor<Self>,
                )
            }
        });
        quote! {
            /// Field-level hot-reload watcher for the `#[config(watch)]`
            /// fields of this struct.
            ///
            /// Feed it the configuration watch channel; `changed()` then
            /// reports exactly which subscribed fields differed between
            /// snapshots. Requires the `watch` feature.
            pub fn field_watcher(
                &self,
                rx: confers::watcher::WatchReceiver<Self>,
            ) -> confers::watcher::StructFieldWatcher<Self> {
                confers::watcher::StructFieldWatcher::new(rx, vec![
                    #(#extractors),*
                ])
            }
        }
    };

    if dynamic_methods_is_empty(&fields) && watch_fields.is_empty() {
        return TokenStream::new();
    }

    quote! {
        #[allow(dead_code)]
        impl #struct_ident {
            #(#dynamic_methods)*
            #watcher_method
        }
    }
}

fn dynamic_methods_is_empty(fields: &[(&Ident, &syn::Type, FieldAttrs)]) -> bool {
    !fields.iter().any(|(_, _, f)| f.dynamic)
}
