// pub type Execute =
//     fn(
//         ctx: Context,
//     ) -> std::pin::Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>>;

// pub struct Command {
//     pub name: &'static str,
//     pub triggers: &'static [&'static str],
//     pub category: &'static str,
//     pub help: Option<&'static str>,
//     pub description: Option<&'static str>,
//     pub group_only: bool,
//     pub owner_only: bool,
//     pub execute: Execute,
// }

use proc_macro::TokenStream;
use quote::quote;
use syn::{bracketed, parse_macro_input};

struct CommandConfig {
    category: syn::Expr,
    triggers: Vec<syn::LitStr>,
    help: Option<syn::Expr>,
    description: Option<syn::Expr>,
    group_only: bool,
    owner_only: bool,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            category: syn::parse_quote!(""),
            triggers: Vec::new(),
            description: None,
            help: None,
            group_only: false,
            owner_only: false,
        }
    }
}

impl syn::parse::Parse for CommandConfig {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut config = CommandConfig::default();
        let mut has_category = false;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let key = ident.to_string();

            input.parse::<syn::Token![=]>()?;

            match key.as_str() {
                "triggers" => {
                    let content;
                    bracketed!(content in input);
                    let val: syn::punctuated::Punctuated<syn::LitStr, syn::Token![,]> =
                        syn::punctuated::Punctuated::parse_terminated(&content)?;
                    config.triggers = val.into_iter().collect();
                }
                "description" => {
                    let value: syn::Expr = input.parse()?;
                    config.description = Some(value);
                }
                "help" => {
                    let value: syn::Expr = input.parse()?;
                    config.help = Some(value);
                }
                "category" => {
                    let val: syn::Expr = input.parse()?;
                    config.category = val;
                    has_category = true;
                }
                "group_only" => {
                    let val: syn::LitBool = input.parse()?;
                    config.group_only = val.value;
                }
                "owner_only" => {
                    let val: syn::LitBool = input.parse()?;
                    config.owner_only = val.value;
                }
                _ => return Err(syn::Error::new(ident.span(), "Unknown parameter")),
            }
            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        if config.triggers.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "triggers is required",
            ));
        }

        if !has_category {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "category is required",
            ));
        }

        Ok(config)
    }
}

#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config = parse_macro_input!(attr as CommandConfig);
    let function = parse_macro_input!(item as syn::ItemFn);

    let ident = &function.sig.ident;
    let name = &ident.to_string();

    let cmd_name = syn::Ident::new(
        &format!("{}_COMMAND", name.to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    let triggers = config.triggers;
    let owner_only = config.owner_only;
    let group_only = config.group_only;
    let category = config.category;

    let description = match config.description {
        Some(expr) => quote!(Some(#expr)),
        None => quote!(None),
    };

    let help = match config.help {
        Some(expr) => quote!(Some(#expr)),
        None => quote!(None),
    };

    TokenStream::from(quote! {
          #function

          #[linkme::distributed_slice(viola_core::command::COMMANDS)]
          static #cmd_name: viola_core::Command = viola_core::Command {
              name: #name,
              category: #category,
              group_only: #group_only,
              description: #description,
              help: #help,
              owner_only: #owner_only,
              triggers: &[#(#triggers),*],
              execute: |ctx: viola_core::Context| Box::pin(#ident(ctx))
        };
    })
}
