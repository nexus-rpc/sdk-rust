mod handler;
mod parse;
mod service;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn nexus_service(args: TokenStream, item: TokenStream) -> TokenStream {
    service::expand_nexus_service(args.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn nexus_operation(args: TokenStream, item: TokenStream) -> TokenStream {
    service::validate_nexus_operation_usage(args.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn nexus_service_handler(args: TokenStream, item: TokenStream) -> TokenStream {
    handler::expand_nexus_service_handler(args.into(), item.into()).into()
}

#[proc_macro_attribute]
pub fn nexus_operation_handler(args: TokenStream, item: TokenStream) -> TokenStream {
    handler::validate_nexus_operation_handler_usage(args.into(), item.into()).into()
}
