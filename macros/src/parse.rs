use darling::{FromDeriveInput, FromField};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(config))]
pub struct ConfigOpts {
    pub ident: syn::Ident,
    #[darling(default)]
    pub env_prefix: Option<String>,
    #[darling(default)]
    pub strict: Option<bool>,
    #[darling(default)]
    pub watch: Option<bool>,
    #[darling(default)]
    pub format_detection: Option<String>,
    #[darling(default)]
    pub audit_log: Option<bool>,
    #[darling(default)]
    pub audit_log_path: Option<String>,
    #[darling(default)]
    pub validate: Option<ValidateOpt>,
    #[darling(default)]
    pub remote: Option<String>,
    #[darling(default)]
    pub remote_timeout: Option<String>,
    #[darling(default)]
    pub remote_fallback: Option<bool>,
    #[darling(default)]
    pub remote_username: Option<String>,
    #[darling(default)]
    pub remote_password: Option<String>,
    #[darling(default)]
    pub remote_token: Option<String>,
    #[darling(default)]
    pub remote_ca_cert: Option<String>,
    #[darling(default)]
    pub remote_client_cert: Option<String>,
    #[darling(default)]
    pub remote_client_key: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidateOpt(pub bool);

impl quote::ToTokens for ValidateOpt {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {}
}

impl darling::FromMeta for ValidateOpt {
    fn from_meta(meta: &syn::Meta) -> Result<Self, darling::Error> {
        match &meta {
            syn::Meta::Path(_) => Ok(ValidateOpt(true)),
            syn::Meta::List(list) => {
                if list.tokens.is_empty() {
                    Ok(ValidateOpt(true))
                } else {
                    let value = list.tokens.clone();
                    if let Ok(ident) = syn::parse2::<syn::Ident>(value) {
                        if ident == "true" {
                            Ok(ValidateOpt(true))
                        } else if ident == "false" {
                            Ok(ValidateOpt(false))
                        } else {
                            Ok(ValidateOpt(true))
                        }
                    } else {
                        Ok(ValidateOpt(true))
                    }
                }
            }
            syn::Meta::NameValue(name_value) => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Bool(lit_bool),
                    ..
                }) = &name_value.value
                {
                    Ok(ValidateOpt(lit_bool.value))
                } else if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit_str),
                    ..
                }) = &name_value.value
                {
                    if lit_str.value() == "true" {
                        Ok(ValidateOpt(true))
                    } else {
                        Ok(ValidateOpt(false))
                    }
                } else {
                    Ok(ValidateOpt(true))
                }
            }
        }
    }
}

impl quote::ToTokens for ConfigOpts {
    fn to_tokens(&self, _tokens: &mut proc_macro2::TokenStream) {
        // No longer used in quote! macro in codegen.rs
    }
}

#[derive(Debug, FromField)]
#[darling(attributes(config), forward_attrs(serde, schemars, validate))]
pub struct FieldOpts {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,
    pub attrs: Vec<syn::Attribute>,
    #[darling(default)]
    pub description: Option<String>,
    #[darling(default)]
    pub default: Option<syn::Expr>,
    #[darling(default)]
    pub flatten: bool,
    #[darling(skip)]
    pub serde_flatten: bool,
    #[darling(default)]
    pub skip: bool,
    #[darling(default)]
    pub name_config: Option<String>,
    #[darling(default)]
    pub name_env: Option<String>,
    #[darling(default)]
    pub name_clap_long: Option<String>,
    #[darling(default)]
    pub name_clap_short: Option<char>,
    #[darling(default)]
    #[allow(dead_code)]
    pub validate: Option<String>,
    #[darling(default)]
    pub custom_validate: Option<String>,
    #[darling(default)]
    pub sensitive: Option<bool>,
    #[darling(default)]
    pub remote: Option<String>,
    #[darling(default)]
    pub remote_timeout: Option<String>,
    #[darling(default)]
    pub remote_auth: Option<bool>,
    #[darling(default)]
    pub remote_username: Option<String>,
    #[darling(default)]
    pub remote_password: Option<String>,
    #[darling(default)]
    pub remote_token: Option<String>,
    #[darling(default)]
    pub remote_tls: Option<bool>,
    #[darling(default)]
    pub remote_ca_cert: Option<String>,
    #[darling(default)]
    pub remote_client_cert: Option<String>,
    #[darling(default)]
    pub remote_client_key: Option<String>,
}
