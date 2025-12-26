use darling::{FromDeriveInput, FromField};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod codegen;
mod parse;

fn has_serde_flatten(attrs: &Vec<syn::Attribute>) -> bool {
    for attr in attrs {
        if attr.path().is_ident("serde") {
            if let syn::Meta::List(list) = &attr.meta {
                let s = list.tokens.to_string();
                // Simple check for "flatten" in the token stream
                if s.contains("flatten") {
                    return true;
                }
            }
        }
    }
    false
}

fn has_validate_derive(input: &syn::DeriveInput) -> bool {
    eprintln!(
        "DEBUG: has_validate_derive called for struct: {}",
        input.ident
    );
    eprintln!("DEBUG: All attributes: {:?}", input.attrs);

    for attr in &input.attrs {
        eprintln!("DEBUG: Checking attribute: {:?}", attr.path());
        if attr.path().is_ident("derive") {
            eprintln!("DEBUG: Found derive attribute");
            eprintln!("DEBUG: Full attribute: {:?}", attr);
            if let syn::Meta::List(list) = &attr.meta {
                // Convert tokens to string and check for Validate
                let tokens_str = list.tokens.to_string();
                eprintln!("DEBUG: Derive tokens string: {}", tokens_str);

                // Simple string check for Validate in the tokens
                if tokens_str.contains("Validate") {
                    eprintln!("DEBUG: Found Validate in derive list!");
                    return true;
                }
            }
        } else if attr.path().is_ident("config") {
            // Skip config attributes - they are not derive attributes
            continue;
        } else {
            // For other attributes, check if they might contain derive information
            eprintln!("DEBUG: Other attribute: {:?}", attr);
        }
    }
    eprintln!("DEBUG: Validate not found in derive list");
    false
}

#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let opts = match parse::ConfigOpts::from_derive_input(&input) {
        Ok(val) => val,
        Err(err) => return err.write_errors().into(),
    };

    let fields = match &input.data {
        syn::Data::Struct(data) => {
            let mut field_opts = Vec::new();
            for field in &data.fields {
                let mut f = match parse::FieldOpts::from_field(field) {
                    Ok(f) => f,
                    Err(e) => return e.write_errors().into(),
                };

                // Check if field has serde(flatten) and set flatten flag
                if has_serde_flatten(&field.attrs) {
                    f.serde_flatten = true;
                    f.flatten = true;
                }
                field_opts.push(f);
            }
            field_opts
        }
        _ => {
            return syn::Error::new_spanned(input, "Config can only be derived for structs")
                .to_compile_error()
                .into()
        }
    };

    let has_validate = has_validate_derive(&input);
    codegen::generate_impl(&opts, &fields, has_validate).into()
}
