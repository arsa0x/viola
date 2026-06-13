use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, ItemFn, LitBool, LitInt, LitStr, Token, bracketed, parse::Parse,
    parse_macro_input, punctuated::Punctuated,
};

/*
* pub struct Command {
*   pub triggers: &'static [&'static str],
*   pub description: &'static str,
*   pub cooldown: Duration,
*   pub owner: bool,
*   pub group_only: bool,
*   pub handler: CommandHandler,
* }
*
* #[command(
*   trigger = [&str],
*   owner = bool,       // false
*   group_only = bool,  // false
*   cooldown u64,       // 0
*   description = &str  // ""
*   help = &str         // ""
* )]
* async fn function(ctx: Context) -> anyhow::Result<()> {
*   Ok(())
* }
*/

struct CommandConfig {
    triggers: Vec<LitStr>,
    help: Option<syn::Expr>,
    description: Option<syn::Expr>,
    cooldown: u64,
    owner: bool,
    group_only: bool,
}

impl Default for CommandConfig {
    fn default() -> Self {
        CommandConfig {
            triggers: Vec::new(),
            description: None,
            help: None,
            cooldown: 0,
            owner: false,
            group_only: false,
        }
    }
}

impl Parse for CommandConfig {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut config = CommandConfig::default();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;

            let key = ident.to_string();

            input.parse::<Token![=]>()?;

            match key.as_str() {
                "trigger" => {
                    let content;
                    bracketed!(content in input);
                    let values: Punctuated<LitStr, Token![,]> =
                        Punctuated::parse_terminated(&content)?;
                    config.triggers = values.into_iter().collect();
                }
                "description" => {
                    let value: Expr = input.parse()?;
                    config.description = Some(value);
                }
                "help" => {
                    let value: syn::Expr = input.parse()?;
                    config.help = Some(value);
                }
                "cooldown" => {
                    let value: LitInt = input.parse()?;
                    config.cooldown = value.base10_parse::<u64>()?;
                }

                "owner" => {
                    let value: LitBool = input.parse()?;
                    config.owner = value.value;
                }

                "group_only" => {
                    let value: LitBool = input.parse()?;
                    config.group_only = value.value;
                }
                _ => {
                    return Err(syn::Error::new(ident.span(), "Unknown parameter"));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        if config.triggers.is_empty() {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "trigger is required",
            ));
        }

        Ok(config)
    }
}

#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config = parse_macro_input!(attr as CommandConfig);
    let function = parse_macro_input!(item as ItemFn);

    let ident = &function.sig.ident;

    let name = &ident.to_string();

    let cmd_name = syn::Ident::new(
        &format!("{}_COMMAND", name.to_uppercase()),
        proc_macro2::Span::call_site(),
    );

    let triggers = config.triggers;

    let owner = config.owner;

    let group_only = config.group_only;

    let cooldown = config.cooldown;

    let description = config.description.unwrap_or_else(|| syn::parse_quote!(""));

    let help = config.help.unwrap_or_else(|| syn::parse_quote!(""));

    // let expanded = quote! {
    //     #function

    //     inventory::submit! {
    //         viola_core::command::Command {
    //             name: #name,
    //             triggers: &[#(#triggers),*],
    //             description: #description,
    //             help: #help,
    //             cooldown: std::time::Duration::from_millis(#cooldown),
    //             owner: #owner,
    //             group_only: #group_only,
    //             handler: |ctx| Box::pin(#ident(ctx)),
    //         }
    //     }
    // };

    let expanded = quote! {
        #function

        #[linkme::distributed_slice(viola_core::command::COMMANDS)]
        static #cmd_name: viola_core::command::Command = viola_core::command::Command {
            name: #name,
            triggers: &[#(#triggers),*],
            description: #description,
            help: #help,
            cooldown: std::time::Duration::from_millis(#cooldown),
            owner: #owner,
            group_only: #group_only,
            handler: |ctx| Box::pin( async move { #ident(ctx).await }),
        };
    };
    TokenStream::from(expanded)
}
