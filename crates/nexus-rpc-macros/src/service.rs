use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::ext::IdentExt;
use syn::{Attribute, Error, ItemTrait, LitStr, Path, TraitItemFn, Visibility};

use crate::parse::{
    combine_errors, error_tokens, extract_operation_signature, parse_methods_from_trait,
    parse_name_override, runtime_crate_path, strip_attr, validate_trait,
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
    let trait_item: ItemTrait = syn::parse2(item)?;
    validate_trait(&trait_item)?;

    let service_name = parse_name_override(args)?
        .unwrap_or_else(|| LitStr::new(&trait_item.ident.to_string(), trait_item.ident.span()));
    if service_name.value().is_empty() {
        return Err(Error::new_spanned(
            service_name,
            "service wire name cannot be empty",
        ));
    }

    let runtime = runtime_crate_path();
    let operations = build_operations(&trait_item, &runtime)?;
    check_duplicate_operation_names(&operations)?;

    let vis = trait_item.vis.clone();
    let marker_vis: Visibility = match &vis {
        Visibility::Inherited => syn::parse_quote!(pub(super)),
        other => other.clone(),
    };
    let trait_ident = trait_item.ident.clone();
    let service_attrs = trait_item.attrs.clone();
    let conditional_attrs = trait_item
        .attrs
        .iter()
        .filter(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        })
        .collect::<Vec<_>>();
    let helper_module = format_ident!("__nexus_service_metadata_for_{}", trait_ident.unraw());

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
                pub static #info_ident: #runtime::OperationDefinitionInfo =
                    #runtime::__macro_support::operation_definition_info(
                        #rust_name,
                        #wire_name,
                        ::core::stringify!(#input_ty),
                        ::core::stringify!(#output_ty),
                    );
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
                    #runtime::__macro_support::operation_definition(&#info_ident);
            })
        })
        .collect::<syn::Result<_>>()?;

    let operation_markers: Vec<_> = operations
        .iter()
        .map(|op| {
            let method_ident = &op.method.sig.ident;
            let marker_ident = format_ident!("__NexusOperation_{}", method_ident.unraw());
            let wire_name = &op.wire_name;
            let (input_ty, output_ty) = extract_operation_signature(&op.method)?;
            Ok(quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #[derive(Clone, Copy, Debug)]
                #marker_vis struct #marker_ident;

                #( #conditional_attrs )*
                impl #runtime::NexusOperation for #marker_ident {
                    type Service = super::#trait_ident;
                    type Input = #input_ty;
                    type Output = #output_ty;

                    fn name(&self) -> &str {
                        #wire_name
                    }
                }
            })
        })
        .collect::<syn::Result<_>>()?;

    let operation_constants: Vec<_> = operations
        .iter()
        .map(|op| {
            let method_ident = &op.method.sig.ident;
            let marker_ident = format_ident!("__NexusOperation_{}", method_ident.unraw());
            quote! {
                #[doc = concat!("Typed definition marker for the `", stringify!(#method_ident), "` operation.")]
                #[allow(non_upper_case_globals)]
                pub const #method_ident: #helper_module::#marker_ident =
                    #helper_module::#marker_ident;
            }
        })
        .collect();

    let tokens = quote! {
        #( #service_attrs )*
        #[doc = concat!("Typed definition marker for the `", stringify!(#trait_ident), "` Nexus service.")]
        #[derive(Clone, Copy, Debug)]
        #vis struct #trait_ident;

        #( #conditional_attrs )*
        impl #runtime::NexusService for #trait_ident {
            fn name() -> &'static str {
                #service_name
            }
        }

        #( #conditional_attrs )*
        impl #runtime::NexusServiceMetadata for #trait_ident {
            fn definition() -> &'static #runtime::ServiceDefinition {
                &#helper_module::__NEXUS_SERVICE_DEFINITION
            }
        }

        #( #conditional_attrs )*
        impl #trait_ident {
            #( #operation_constants )*
        }

        #[doc(hidden)]
        #[allow(non_snake_case, non_camel_case_types, missing_docs)]
        #( #conditional_attrs )*
        #vis mod #helper_module {
            use super::*;

            #( #operation_infos )*

            #[doc(hidden)]
            pub static __NEXUS_OPERATION_INFOS: &[#runtime::OperationDefinitionInfo] =
                &[ #( #info_names ),* ];

            #[doc(hidden)]
            pub static __NEXUS_SERVICE_DEFINITION: #runtime::ServiceDefinition =
                #runtime::__macro_support::service_definition(
                    #service_name,
                    __NEXUS_OPERATION_INFOS,
                );

            #( #operation_defs )*
            #( #operation_markers )*
        }
    };

    Ok(tokens)
}

fn build_operations(item: &ItemTrait, runtime: &Path) -> syn::Result<Vec<OperationSpec>> {
    let methods = parse_methods_from_trait(item)?;
    methods
        .into_iter()
        .map(|method| {
            let has_operation = method
                .attrs
                .iter()
                .any(|attribute| is_runtime_attribute(attribute, runtime, "operation"));
            let has_legacy_operation = method
                .attrs
                .iter()
                .any(|attribute| is_runtime_attribute(attribute, runtime, "nexus_operation"));
            if has_operation && has_legacy_operation {
                return Err(Error::new_spanned(
                    &method.sig.ident,
                    "a service method cannot use both `#[operation]` and `#[nexus_operation]`",
                ));
            }
            let wire_name = if has_operation {
                parse_runtime_name(&method.attrs, runtime, "operation")?
            } else if has_legacy_operation {
                parse_runtime_name(&method.attrs, runtime, "nexus_operation")?
            } else {
                None
            }
            .unwrap_or_else(|| LitStr::new(&method.sig.ident.to_string(), method.sig.ident.span()));
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

fn parse_runtime_name(
    attributes: &[Attribute],
    runtime: &Path,
    name: &str,
) -> syn::Result<Option<LitStr>> {
    let attributes = attributes
        .iter()
        .filter(|attribute| is_runtime_attribute(attribute, runtime, name))
        .collect::<Vec<_>>();
    if attributes.len() > 1 {
        return Err(Error::new_spanned(
            attributes[1],
            format!("duplicate `#[{name}]` attribute"),
        ));
    }
    let Some(attribute) = attributes.first() else {
        return Ok(None);
    };
    if attribute.meta.require_list().is_err() && !matches!(attribute.meta, syn::Meta::Path(_)) {
        return Err(Error::new_spanned(attribute, "malformed attribute"));
    }
    parse_name_override(
        attribute
            .meta
            .require_list()
            .map(|list| list.tokens.clone())
            .unwrap_or_default(),
    )
}

fn is_runtime_attribute(attribute: &Attribute, runtime: &Path, name: &str) -> bool {
    let path = attribute.path();
    if path.is_ident(name) {
        return true;
    }
    let Some(runtime_ident) = runtime.segments.last().map(|segment| &segment.ident) else {
        return false;
    };
    path.segments.len() == 2
        && path.segments.first().map(|segment| &segment.ident) == Some(runtime_ident)
        && path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
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

pub(crate) fn validate_operation_usage(args: TokenStream, item: TokenStream) -> TokenStream {
    if let Err(error) = expect_no_args_or_name(args, "operation") {
        return error_tokens(error);
    }

    match syn::parse2::<TraitItemFn>(item.clone()) {
        Ok(mut method) => {
            method.attrs = strip_attr(&method.attrs, "operation");
            quote!(#method)
        }
        Err(_) => error_tokens(Error::new(
            Span::call_site(),
            "`#[operation]` can only be used on service trait methods",
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
