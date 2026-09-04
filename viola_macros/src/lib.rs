use proc_macro::TokenStream;
use quote::quote;
use syn::{bracketed, parse_macro_input};

struct CommandConfig {
    category: syn::Expr,
    triggers: Vec<syn::LitStr>,
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

/// Registers an asynchronous function as a Viola command.
///
/// The `command` attribute transforms the annotated function into a
/// distributed [`viola_core::Command`] definition and registers it in
/// `viola_core::command::COMMANDS`.
///
/// The annotated function itself is preserved and used as the command's
/// execution handler.
///
/// # Syntax
///
/// ```ignore
/// #[viola_macros::command(
///     triggers = ["trigger", "alias"],
///     category = "category",
///     description = "Optional command description",
///     group_only = false,
///     owner_only = false,
/// )]
/// async fn my_command(ctx: viola_core::Context) -> anyhow::Result<()> {
///     // command implementation
///     Ok(())
/// }
/// ```
///
/// # Parameters
///
/// ## `triggers`
///
/// A non-empty list of string literals used to invoke the command.
///
/// Multiple triggers can be provided to create aliases for the same command.
///
/// ```ignore
/// triggers = ["nekopoi", "neko", "nkp"]
/// ```
///
/// ## `category`
///
/// Specifies the command category. This parameter is required.
///
/// ```ignore
/// category = "utility"
/// ```
///
/// ## `description`
///
/// An optional expression containing a short description of the command.
///
/// ```ignore
/// description = "Search for something"
/// ```
///
/// ## `group_only`
///
/// When set to `true`, the command can only be executed in group chats.
///
/// Defaults to `false`.
///
/// ```ignore
/// group_only = true
/// ```
///
/// ## `owner_only`
///
/// When set to `true`, the command can only be executed by configured
/// bot owners.
///
/// Defaults to `false`.
///
/// ```ignore
/// owner_only = true
/// ```
///
/// # Requirements
///
/// The annotated function must be an asynchronous function accepting a
/// [`viola_core::Context`] and returning an `anyhow::Result<()>`.
///
/// ```ignore
/// async fn example(ctx: viola_core::Context) -> anyhow::Result<()> {
///     Ok(())
/// }
/// ```
///
/// Both `triggers` and `category` are required. Compilation fails with a
/// descriptive error if either parameter is omitted.
///
/// # Registration
///
/// Internally, this macro generates a static [`viola_core::Command`] value
/// and registers it in the `viola_core::command::COMMANDS` distributed slice
/// using `linkme`.
///
/// This allows commands to be discovered and registered automatically
/// without requiring each command to be manually added to a central list.
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

    TokenStream::from(quote! {
          #function

          #[linkme::distributed_slice(viola_core::command::COMMANDS)]
          static #cmd_name: viola_core::Command = viola_core::Command {
              name: #name,
              category: #category,
              group_only: #group_only,
              description: #description,
              owner_only: #owner_only,
              triggers: &[#(#triggers),*],
              execute: |ctx: viola_core::Context| Box::pin(#ident(ctx))
        };
    })
}
