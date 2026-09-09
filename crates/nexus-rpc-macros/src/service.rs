use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{Error, ItemTrait, LitStr, TraitItem, TraitItemFn};

use crate::parse::{
    combine_errors, error_tokens, extract_operation_signature, parse_methods_from_trait,
    parse_name_list, parse_name_override, runtime_crate_path, strip_attr, validate_trait,
};

struct OperationSpec {
    method: TraitItemFn,
    wire_name: LitStr,
}

pub(crate) fn expand_nexus_service(args: TokenStream, item: TokenStream) -> TokenStream {
    match expand_nexus_service_inner(args, item) {
        Ok(tokens) => tokens,
        Err(error) => error_tokens(error),
    }
}

fn expand_nexus_service_inner(args: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let mut trait_item: ItemTrait = syn::parse2(item)?;
    validate_trait(&trait_item)?;

    let service_name = parse_name_override(args)?
        .unwrap_or_else(|| LitStr::new(&trait_item.ident.to_string(), trait_item.ident.span()));
    if service_name.value().is_empty() {
        return Err(Error::new_spanned(
            service_name,
            "service wire name cannot be empty",
        ));
    }

    let operations = build_operations(&trait_item)?;
    check_duplicate_operation_names(&operations)?;

    for item in &mut trait_item.items {
        if let TraitItem::Fn(method) = item {
            method.attrs = strip_attr(&method.attrs, "nexus_operation");
        }
    }

    let vis = trait_item.vis.clone();
    let trait_ident = trait_item.ident.clone();
    let helper_module = format_ident!("__nexus_service_metadata_for_{}", trait_ident.unraw());
    let runtime = runtime_crate_path();
    let binding_trait_names: Vec<_> = operations
        .iter()
        .map(|op| format_ident!("__NexusBind_{}", op.method.sig.ident.unraw()))
        .collect();
    let input_type_names: Vec<_> = operations
        .iter()
        .map(|op| format_ident!("__NexusInput_{}", op.method.sig.ident.unraw()))
        .collect();
    let output_type_names: Vec<_> = operations
        .iter()
        .map(|op| format_ident!("__NexusOutput_{}", op.method.sig.ident.unraw()))
        .collect();

    let info_names: Vec<_> = operations
        .iter()
        .enumerate()
        .map(|(index, _)| format_ident!("__NEXUS_INFO_{index}"))
        .collect();
    let def_names: Vec<_> = operations
        .iter()
        .enumerate()
        .map(|(index, _)| format_ident!("__NEXUS_DEF_{index}"))
        .collect();

    let operation_infos: Vec<_> = operations
        .iter()
        .zip(info_names.iter())
        .map(|(op, info_ident)| {
            let rust_name = LitStr::new(
                &op.method.sig.ident.unraw().to_string(),
                op.method.sig.ident.span(),
            );
            let wire_name = &op.wire_name;
            let (input_ty, output_ty) = extract_operation_signature(&op.method)?;
            Ok(quote! {
                #[doc(hidden)]
                pub static #info_ident: #runtime::integration::OperationDefinitionInfo =
                    #runtime::integration::OperationDefinitionInfo::new(
                        #rust_name,
                        #wire_name,
                        ::core::stringify!(#input_ty),
                        ::core::stringify!(#output_ty),
                    );
            })
        })
        .collect::<syn::Result<_>>()?;

    let operation_type_aliases: Vec<_> = operations
        .iter()
        .zip(input_type_names.iter().zip(output_type_names.iter()))
        .map(|(op, (input_ident, output_ident))| {
            let (input_ty, output_ty) = extract_operation_signature(&op.method)?;
            Ok(quote! {
                #[doc(hidden)]
                pub type #input_ident = #input_ty;

                #[doc(hidden)]
                pub type #output_ident = #output_ty;
            })
        })
        .collect::<syn::Result<_>>()?;

    let operation_defs: Vec<_> = operations
        .iter()
        .zip(info_names.iter().zip(def_names.iter()))
        .map(|(op, (info_ident, def_ident))| {
            let (input_ty, output_ty) = extract_operation_signature(&op.method)?;
            Ok(quote! {
                #[doc(hidden)]
                pub static #def_ident: #runtime::OperationDefinition<#input_ty, #output_ty> =
                    #runtime::OperationDefinition::new(&#info_ident);
            })
        })
        .collect::<syn::Result<_>>()?;

    let binding_traits: Vec<_> = operations
        .iter()
        .zip(
            binding_trait_names
                .iter()
                .zip(input_type_names.iter().zip(output_type_names.iter())),
        )
        .map(|(op, (binding_trait, (input_ident, output_ident)))| {
            let method_ident = &op.method.sig.ident;
            Ok(quote! {
                #[doc(hidden)]
                pub trait #binding_trait {
                    fn #method_ident(
                        &self,
                    ) -> impl #runtime::OperationHandler<#input_ident, #output_ident> + '_;
                }
            })
        })
        .collect::<syn::Result<_>>()?;

    let registered_wrappers: Vec<_> = operations
        .iter()
        .zip(binding_trait_names.iter().zip(def_names.iter()))
        .map(|(op, (binding_trait, def_ident))| {
            let method_ident = &op.method.sig.ident;
            let registered_ident = format_ident!("__NexusRegistered_{}", method_ident.unraw());
            let (input_ty, output_ty) = extract_operation_signature(&op.method)?;
            Ok(quote! {
                #[doc(hidden)]
                pub struct #registered_ident<S> {
                    service: ::std::sync::Arc<S>,
                }

                impl<S> #runtime::OperationHandler<#input_ty, #output_ty> for #registered_ident<S>
                where
                    S: #binding_trait + ::core::marker::Send + ::core::marker::Sync + 'static,
                {
                    async fn start(
                        &self,
                        context: #runtime::OperationStartContext,
                        input: #input_ty,
                    ) -> ::core::result::Result<
                        #runtime::OperationStartResult<#output_ty>,
                        #runtime::BoxError,
                    > {
                        let handler = <S as #binding_trait>::#method_ident(self.service.as_ref());
                        handler.start(context, input).await
                    }

                    async fn cancel(
                        &self,
                        context: #runtime::OperationCancelContext,
                    ) -> ::core::result::Result<(), #runtime::BoxError> {
                        let handler = <S as #binding_trait>::#method_ident(self.service.as_ref());
                        handler.cancel(context).await
                    }
                }

                #[doc(hidden)]
                pub fn #method_ident<S, R>(
                    service: &::std::sync::Arc<S>,
                    registrar: &mut R,
                ) where
                    S: #binding_trait + ::core::marker::Send + ::core::marker::Sync + 'static,
                    R: #runtime::integration::OperationRegistrar,
                    #input_ty: ::core::marker::Send + 'static,
                    #output_ty: ::core::marker::Send + 'static,
                {
                    registrar.register(
                        &#def_ident,
                        #registered_ident {
                            service: ::std::sync::Arc::clone(service),
                        },
                    );
                }
            })
        })
        .collect::<syn::Result<_>>()?;

    let registration_calls: Vec<_> = operations
        .iter()
        .map(|op| {
            let method_ident = &op.method.sig.ident;
            quote!(#method_ident(&service, registrar);)
        })
        .collect();

    let all_bindings = if binding_trait_names.is_empty() {
        quote! {
            #[doc(hidden)]
            pub trait __NexusAllBindings {}
            impl<T> __NexusAllBindings for T {}
        }
    } else {
        quote! {
            #[doc(hidden)]
            pub trait __NexusAllBindings: #( #binding_trait_names + )* {}

            impl<T> __NexusAllBindings for T where T: #( #binding_trait_names + )* {}
        }
    };

    let tokens = quote! {
        #trait_item

        #[doc(hidden)]
        #[allow(non_snake_case, non_camel_case_types, missing_docs)]
        #vis mod #helper_module {
            use super::*;

            #( #operation_infos )*

            #[doc(hidden)]
            pub static __NEXUS_OPERATION_INFOS: &[#runtime::integration::OperationDefinitionInfo] =
                &[ #( #info_names ),* ];

            #[doc(hidden)]
            pub static __NEXUS_SERVICE_DEFINITION: #runtime::ServiceDefinition =
                #runtime::ServiceDefinition::new(#service_name, __NEXUS_OPERATION_INFOS);

            #( #operation_type_aliases )*
            #( #operation_defs )*
            #( #binding_traits )*
            #all_bindings
            #( #registered_wrappers )*

            #[doc(hidden)]
            pub fn register<S, R>(service: S, registrar: &mut R)
            where
                S: __NexusAllBindings + ::core::marker::Send + ::core::marker::Sync + 'static,
                R: #runtime::integration::OperationRegistrar,
            {
                let service = ::std::sync::Arc::new(service);
                #( #registration_calls )*
            }
        }
    };

    Ok(tokens)
}

fn build_operations(item: &ItemTrait) -> syn::Result<Vec<OperationSpec>> {
    let methods = parse_methods_from_trait(item)?;
    methods
        .into_iter()
        .map(|method| {
            let wire_name =
                parse_name_list(&method.attrs, "nexus_operation")?.unwrap_or_else(|| {
                    LitStr::new(&method.sig.ident.to_string(), method.sig.ident.span())
                });
            if wire_name.value().is_empty() {
                return Err(Error::new_spanned(
                    wire_name,
                    "operation wire name cannot be empty",
                ));
            }
            Ok(OperationSpec { method, wire_name })
        })
        .collect()
}

fn check_duplicate_operation_names(operations: &[OperationSpec]) -> syn::Result<()> {
    let mut errors = Vec::new();
    let mut seen = std::collections::BTreeMap::<String, Span>::new();
    for operation in operations {
        let wire_name = operation.wire_name.value();
        if let Some(previous) = seen.insert(wire_name.clone(), operation.wire_name.span()) {
            errors.push(Error::new(
                operation.wire_name.span(),
                format!("duplicate operation wire name `{wire_name}`"),
            ));
            errors.push(Error::new(
                previous,
                "previous operation with the same wire name",
            ));
        }
    }
    combine_errors(errors)
}

pub(crate) fn validate_nexus_operation_usage(args: TokenStream, item: TokenStream) -> TokenStream {
    if let Err(error) = expect_no_args_or_name(args, "nexus_operation") {
        return error_tokens(error);
    }

    match syn::parse2::<TraitItemFn>(item.clone()) {
        Ok(mut method) => {
            method.attrs = strip_attr(&method.attrs, "nexus_operation");
            quote!(#method)
        }
        Err(_) => error_tokens(Error::new(
            Span::call_site(),
            "`#[nexus_operation]` can only be used on service trait methods",
        )),
    }
}

fn expect_no_args_or_name(args: TokenStream, label: &str) -> syn::Result<()> {
    if args.is_empty() {
        return Ok(());
    }
    let name = parse_name_override(args)?;
    if name.is_none() {
        return Err(Error::new(
            Span::call_site(),
            format!("{label} only accepts `name = \"...\"`"),
        ));
    }
    Ok(())
}
