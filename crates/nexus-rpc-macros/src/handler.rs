use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{Error, ImplItem, ImplItemFn, ItemImpl};

use crate::parse::{
    combine_errors, error_tokens, expect_no_args, is_temporal_operation_attr,
    parse_service_handler_args, path_ends_with_ident, runtime_crate_path, strip_attr,
    trait_path_to_helper_module, validate_provider_method,
};

pub(crate) fn expand_nexus_service_handler(args: TokenStream, item: TokenStream) -> TokenStream {
    match expand_nexus_service_handler_inner(args, item) {
        Ok(tokens) => tokens,
        Err(error) => error_tokens(error),
    }
}

fn expand_nexus_service_handler_inner(
    args: TokenStream,
    item: TokenStream,
) -> syn::Result<TokenStream> {
    let service_args = parse_service_handler_args(args)?;
    let mut impl_item: ItemImpl = syn::parse2(item)?;

    if impl_item.trait_.is_some() {
        return Err(Error::new_spanned(
            &impl_item.self_ty,
            "`#[nexus_service_handler]` only supports inherent impl blocks",
        ));
    }
    if !impl_item.generics.params.is_empty() || impl_item.generics.where_clause.is_some() {
        return Err(Error::new_spanned(
            &impl_item.generics,
            "`#[nexus_service_handler]` does not support impl generics",
        ));
    }

    let helper_module = trait_path_to_helper_module(&service_args.service)?;
    let self_ty = (*impl_item.self_ty).clone();
    let runtime = runtime_crate_path();

    let mut provider_impls = Vec::new();
    let mut provider_assertions = Vec::new();
    let mut errors = Vec::new();

    for item in &mut impl_item.items {
        let ImplItem::Fn(method) = item else {
            continue;
        };

        let has_marker = method
            .attrs
            .iter()
            .any(|attr| path_ends_with_ident(attr.path(), "nexus_operation_handler"));
        let is_extension_candidate =
            !has_marker && method.attrs.iter().any(is_temporal_operation_attr);

        if !has_marker && !is_extension_candidate {
            continue;
        }

        if has_marker && let Err(error) = validate_provider_method(method) {
            errors.push(error);
            continue;
        }

        method.attrs = strip_attr(&method.attrs, "nexus_operation_handler");

        let method_ident = method.sig.ident.clone();
        let binding_trait = format_ident!("__NexusBind_{}", method_ident.unraw());
        let input_ident = format_ident!("__NexusInput_{}", method_ident.unraw());
        let output_ident = format_ident!("__NexusOutput_{}", method_ident.unraw());
        provider_impls.push(quote! {
            impl #helper_module::#binding_trait for #self_ty {
                fn #method_ident(&self) -> impl #runtime::OperationHandler<
                    #helper_module::#input_ident,
                    #helper_module::#output_ident
                > + '_ {
                    #self_ty::#method_ident(self)
                }
            }
        });
        provider_assertions.push(quote! {
            let _ = <#self_ty as #helper_module::#binding_trait>::#method_ident;
        });
    }

    combine_errors(errors)?;

    let binding_assert = quote! {
        const _: fn() = || {
            fn __assert_bindings<T>()
            where
                T: #helper_module::__NexusAllBindings,
            {
            }
            __assert_bindings::<#self_ty>();
            #( #provider_assertions )*
        };
    };

    let tokens = quote! {
        #impl_item

        #( #provider_impls )*

        impl #runtime::ServiceHandler for #self_ty
        where
            #self_ty: #helper_module::__NexusAllBindings,
        {
            fn definition() -> &'static #runtime::ServiceDefinition {
                &#helper_module::__NEXUS_SERVICE_DEFINITION
            }

            fn register<R>(self, registrar: &mut R)
            where
                Self: Sized,
                R: #runtime::integration::OperationRegistrar,
            {
                #helper_module::register(self, registrar);
            }
        }

        #binding_assert
    };

    Ok(tokens)
}

pub(crate) fn validate_nexus_operation_handler_usage(
    args: TokenStream,
    item: TokenStream,
) -> TokenStream {
    if let Err(error) = expect_no_args(args, "nexus_operation_handler") {
        return error_tokens(error);
    }

    match syn::parse2::<ImplItemFn>(item.clone()) {
        Ok(mut method) => {
            method.attrs = strip_attr(&method.attrs, "nexus_operation_handler");
            quote!(#method)
        }
        Err(_) => error_tokens(Error::new(
            proc_macro2::Span::call_site(),
            "`#[nexus_operation_handler]` can only be used on inherent methods",
        )),
    }
}
