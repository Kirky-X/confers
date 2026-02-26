//! Clap CLI argument generation for Config derive macro.
//!
//! Generates ClapArgs struct and CLI argument support.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Fields};
use darling::FromField;

use crate::parse::{StructAttrs, FieldAttrs};

/// Generate ClapArgs struct for CLI argument parsing.
pub fn generate_clap_impl(
    struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &Fields,
) -> TokenStream {
    let _env_prefix = attrs.effective_env_prefix();
    let app_name = attrs.app_name.as_deref().unwrap_or("app");

    // Generate field definitions for ClapArgs
    let clap_field_defs: Vec<TokenStream> = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let field_attrs = FieldAttrs::from_field(field).ok()?;
            if field_attrs.skip {
                return None;
            }

            let field_name = field_attrs.effective_name();
            let cli_name = field_attrs.name_clap_long.clone()
                .unwrap_or_else(|| field_name.replace('.', "-"));

            // Build arg attributes
            let mut arg_parts = vec![quote! { long = #cli_name }];

            if let Some(short) = field_attrs.name_clap_short {
                arg_parts.push(quote! { short = #short });
            }

            if let Some(desc) = &field_attrs.description {
                arg_parts.push(quote! { help = #desc });
            }

            // Check if field has a default
            let has_default = field_attrs.default.is_some();

            let ty = &field.ty;
            let type_str = quote!(#ty).to_string();

            // Handle optional types - make them optional in CLI
            if type_str.contains("Option") {
                arg_parts.push(quote! { required = false });
            } else if has_default {
                // Fields with defaults are optional
                arg_parts.push(quote! { required = false });
            }

            let arg_attr = quote! { #[arg(#(#arg_parts),*)] };

            Some(quote! {
                #arg_attr
                pub #ident: #ty
            })
        })
        .collect();

    // Generate field names for to_config_map
    let field_idents: Vec<TokenStream> = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let attrs = FieldAttrs::from_field(field).ok()?;
            if attrs.skip {
                return None;
            }
            Some(quote! { #ident })
        })
        .collect();

    // Create a unique type name based on struct name
    let cli_args_ident = quote::format_ident!("{}CliArgs", struct_ident);

    quote! {
        /// CLI arguments generated from configuration struct.
        ///
        /// # Example
        ///
        /// ```ignore
        /// use clap::Parser;
        ///
        /// #[derive(ConfigClap)]
        /// struct MyConfig {
        ///     #[config(name_clap_long = "host", name_clap_short = 'h')]
        ///     pub host: String,
        /// }
        ///
        /// fn main() {
        ///     let args = <MyConfig as ConfigClap>::clap_args();
        ///     // ... use args
        /// }
        /// ```
        impl #struct_ident {
            /// Generate clap Args struct by parsing command line arguments.
            #[allow(dead_code)]
            pub fn clap_args() -> #cli_args_ident {
                #cli_args_ident::parse()
            }

            /// Get clap app for custom configuration.
            #[allow(dead_code)]
            pub fn clap_app() -> clap::Command {
                <#cli_args_ident as clap::IntoApp>::command()
            }

            /// Create clap args from iterator of strings (for testing).
            #[allow(dead_code)]
            pub fn clap_args_from<I>(iter: I) -> #cli_args_ident
            where
                I: Iterator<Item = std::ffi::OsString>,
            {
                <#cli_args_ident as clap::FromArgMatches>::from_arg_matches(
                    &<#cli_args_ident as clap::IntoApp>::command()
                        .try_get_matches_from(iter)
                        .unwrap()
                )
                .unwrap()
            }
        }

        /// CLI arguments struct (use via ConfigClap trait).
        #[derive(clap::Parser, Debug)]
        #[command(name = #app_name)]
        #[allow(dead_code)]
        pub struct #cli_args_ident {
            #(#clap_field_defs),*
        }

        impl #cli_args_ident {
            /// Convert CLI arguments to a configuration map.
            #[allow(dead_code)]
            pub fn to_config_map(&self) -> std::collections::HashMap<String, confers::ConfigValue> {
                let mut map = std::collections::HashMap::new();
                #(
                    map.insert(
                        stringify!(#field_idents).to_string(),
                        confers::ConfigValue::from(&self.#field_idents)
                    );
                )*
                map
            }
        }
    }
}
