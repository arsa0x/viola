use proc_macro::TokenStream;
use quote::quote;
use syn::{ExprArray, ItemFn, parse_macro_input};

#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let triggers = parse_macro_input!(attr as ExprArray);
    let function = parse_macro_input!(item as ItemFn);
    let name = &function.sig.ident;

    let expanded = quote! {
        #function
        inventory::submit! {
            crate::framework::command::Command {
                triggers: &#triggers,
                handler: |ctx| {
                    Box::pin(#name(ctx))
                }
            }
        }

    };
    TokenStream::from(expanded)
}
