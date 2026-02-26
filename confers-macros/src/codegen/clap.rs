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
    _struct_ident: &Ident,
    attrs: &StructAttrs,
    fields: &Fields,
) -> TokenStream {
    let env_prefix = attrs.effective_env_prefix();
    let clap_args: Vec<TokenStream> = fields
        .iter()
        .filter_map(|field| {
            let ident = field.ident.as_ref()?;
            let attrs = FieldAttrs::from_field(field).ok()?;
            if attrs.skip {
                return None;
            }

            let field_name = attrs.effective_name();
            let cli_name = attrs.name_clap_long.clone()
                .unwrap_or_else(|| field_name.replace('.', "-"));

            // Get type string for CLI argument
            let ty = &field.ty;
            let type_str = quote!(#ty).to_string();

            let arg_attrs = if type_str.contains("bool") {
                // Boolean flags
                quote! {
                    #[arg(long = #cli_name, env = #env_prefix)]
                }
            } else {
                // Regular values
                quote! {
                    #[arg(long = #cli_name, env = #env_prefix)]
                }
            };

            Some(quote! {
                #arg_attrs
                pub #ident: #ty
            })
        })
        .collect();

    quote! {
        /// CLI arguments generated from configuration struct.
        #[derive(clap::Parser, Debug)]
        #[command(name = #env_prefix)]
        pub struct ClapArgs {
            #(#clap_args),*
        }

        impl ClapArgs {
            /// Convert CLI arguments to a configuration map.
            pub fn to_config_map(&self) -> std::collections::HashMap<String, confers::ConfigValue> {
                let mut map = std::collections::HashMap::new();
                // Convert each field to ConfigValue
                #(
                    map.insert(
                        stringify!(#clap_args).to_string(),
                        confers::ConfigValue::from(self.#clap_args.clone())
                    );
                )*
                map
            }
        }
    }
}
