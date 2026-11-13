mod parse;
mod service;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn service(args: TokenStream, item: TokenStream) -> TokenStream {
    service::expand_service(args.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn operation(args: TokenStream, item: TokenStream) -> TokenStream {
    service::validate_operation_usage(args.into(), item.into()).into()
}
