use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Error, Expr, FnArg, Ident, ImplItemFn, ItemTrait, LitStr, MetaNameValue, Path,
    Result, ReturnType, TraitItem, TraitItemFn, Type,
};

pub(crate) struct NameArg {
    pub(crate) name: LitStr,
}

impl Parse for NameArg {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let meta = input.parse::<MetaNameValue>()?;
        if meta.path != syn::parse_quote!(name) {
            return Err(Error::new_spanned(meta.path, "expected `name = \"...\"`"));
        }
        let Expr::Lit(expr_lit) = meta.value else {
            return Err(Error::new_spanned(meta.value, "expected string literal"));
        };
        let syn::Lit::Str(name) = expr_lit.lit else {
            return Err(Error::new_spanned(expr_lit, "expected string literal"));
        };
        if !input.is_empty() {
            return Err(input.error("unexpected extra attribute arguments"));
        }
        Ok(Self { name })
    }
}

pub(crate) struct ServiceHandlerArgs {
    pub(crate) service: Path,
}

impl Parse for ServiceHandlerArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let meta = input.parse::<MetaNameValue>()?;
        if meta.path != syn::parse_quote!(service) {
            return Err(Error::new_spanned(
                meta.path,
                "expected `service = ServiceTrait`",
            ));
        }
        let Expr::Path(expr_path) = meta.value else {
            return Err(Error::new_spanned(meta.value, "expected a trait path"));
        };
        if !input.is_empty() {
            return Err(input.error("unexpected extra attribute arguments"));
        }
        Ok(Self {
            service: expr_path.path,
        })
    }
}

pub(crate) fn parse_name_override(tokens: TokenStream) -> Result<Option<LitStr>> {
    if tokens.is_empty() {
        return Ok(None);
    }
    syn::parse2::<NameArg>(tokens).map(|parsed| Some(parsed.name))
}

pub(crate) fn parse_service_handler_args(tokens: TokenStream) -> Result<ServiceHandlerArgs> {
    syn::parse2(tokens)
}

pub(crate) fn strip_attr(attrs: &[Attribute], attr_name: &str) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| !path_ends_with_ident(attr.path(), attr_name))
        .cloned()
        .collect()
}

pub(crate) fn path_ends_with_ident(path: &Path, ident: &str) -> bool {
    path.segments
        .last()
        .is_some_and(|segment| segment.ident == ident)
}

pub(crate) fn runtime_crate_path() -> Path {
    match crate_name("nexus-rpc") {
        Ok(FoundCrate::Itself) => syn::parse_quote!(::nexus_rpc),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, proc_macro2::Span::call_site());
            syn::parse_quote!(::#ident)
        }
        // This permits macro-only tests and custom runtime shims which expose the
        // conventional crate name without declaring a Cargo dependency.
        Err(_) => syn::parse_quote!(::nexus_rpc),
    }
}

pub(crate) fn is_temporal_operation_attr(attr: &Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "temporal_operation")
}

pub(crate) fn validate_trait(item: &ItemTrait) -> Result<()> {
    let mut errors = Vec::new();
    if !item.generics.params.is_empty() {
        errors.push(Error::new_spanned(
            &item.generics,
            "nexus services do not support generics or lifetimes",
        ));
    }
    if let Some(where_clause) = &item.generics.where_clause {
        errors.push(Error::new_spanned(
            where_clause,
            "nexus services do not support where clauses",
        ));
    }
    if !item.supertraits.is_empty() {
        errors.push(Error::new_spanned(
            &item.supertraits,
            "nexus services do not support supertraits",
        ));
    }

    for item in &item.items {
        match item {
            TraitItem::Fn(method) => validate_service_method(method, &mut errors),
            _ => errors.push(Error::new_spanned(
                item,
                "nexus services only support operation method declarations",
            )),
        }
    }

    combine_errors(errors)
}

fn validate_service_method(method: &TraitItemFn, errors: &mut Vec<Error>) {
    if let Some(default) = &method.default {
        errors.push(Error::new_spanned(
            default,
            "nexus operations cannot have method bodies",
        ));
    }
    if let Some(asyncness) = &method.sig.asyncness {
        errors.push(Error::new_spanned(
            asyncness,
            "nexus operations cannot be async",
        ));
    }
    if !method.sig.generics.params.is_empty() || method.sig.generics.where_clause.is_some() {
        errors.push(Error::new_spanned(
            &method.sig.generics,
            "nexus operations do not support generics or lifetimes",
        ));
    }
    if method.sig.constness.is_some() || method.sig.unsafety.is_some() || method.sig.abi.is_some() {
        errors.push(Error::new_spanned(
            &method.sig,
            "nexus operations must be plain trait methods",
        ));
    }
    if method.sig.inputs.len() != 1 {
        errors.push(Error::new_spanned(
            &method.sig.inputs,
            "nexus operations must take exactly one input type",
        ));
    }
    for input in &method.sig.inputs {
        if matches!(input, FnArg::Receiver(_)) {
            errors.push(Error::new_spanned(
                input,
                "nexus operations cannot take a receiver",
            ));
        }
    }
    if matches!(method.sig.output, ReturnType::Default) {
        errors.push(Error::new_spanned(
            &method.sig.output,
            "nexus operations must return exactly one output type",
        ));
    }
}

pub(crate) fn extract_operation_signature(method: &TraitItemFn) -> Result<(Type, Type)> {
    let input = method
        .sig
        .inputs
        .iter()
        .next()
        .ok_or_else(|| Error::new_spanned(&method.sig.inputs, "missing operation input"))?;
    let FnArg::Typed(input) = input else {
        return Err(Error::new_spanned(
            input,
            "nexus operations cannot take a receiver",
        ));
    };
    let ReturnType::Type(_, output) = &method.sig.output else {
        return Err(Error::new_spanned(
            &method.sig.output,
            "nexus operations must return exactly one output type",
        ));
    };
    Ok(((*input.ty).clone(), (*output).as_ref().clone()))
}

pub(crate) fn validate_provider_method(method: &ImplItemFn) -> Result<()> {
    let mut errors = Vec::new();
    if let Some(asyncness) = &method.sig.asyncness {
        errors.push(Error::new_spanned(
            asyncness,
            "nexus operation providers cannot be async",
        ));
    }
    if !method.sig.generics.params.is_empty() || method.sig.generics.where_clause.is_some() {
        errors.push(Error::new_spanned(
            &method.sig.generics,
            "nexus operation providers do not support generics or lifetimes",
        ));
    }
    if method.sig.inputs.len() != 1 {
        errors.push(Error::new_spanned(
            &method.sig.inputs,
            "nexus operation providers must take only `&self`",
        ));
    }
    match method.sig.inputs.first() {
        Some(FnArg::Receiver(receiver))
            if receiver.reference.is_some() && receiver.mutability.is_none() => {}
        Some(other) => errors.push(Error::new_spanned(
            other,
            "nexus operation providers must take only `&self`",
        )),
        None => errors.push(Error::new_spanned(
            &method.sig,
            "nexus operation providers must take only `&self`",
        )),
    }
    if matches!(method.sig.output, ReturnType::Default) {
        errors.push(Error::new_spanned(
            &method.sig.output,
            "nexus operation providers must return an operation handler",
        ));
    }
    if method.sig.constness.is_some() || method.sig.unsafety.is_some() || method.sig.abi.is_some() {
        errors.push(Error::new_spanned(
            &method.sig,
            "nexus operation providers must be plain inherent methods",
        ));
    }
    combine_errors(errors)
}

pub(crate) fn combine_errors(errors: Vec<Error>) -> Result<()> {
    let mut iter = errors.into_iter();
    match iter.next() {
        Some(mut error) => {
            for next in iter {
                error.combine(next);
            }
            Err(error)
        }
        None => Ok(()),
    }
}

pub(crate) fn expect_no_args(tokens: TokenStream, label: &str) -> Result<()> {
    if tokens.is_empty() {
        Ok(())
    } else {
        Err(Error::new(
            proc_macro2::Span::call_site(),
            format!("{label} does not accept attribute arguments"),
        ))
    }
}

pub(crate) fn parse_name_list(attrs: &[Attribute], attr_name: &str) -> Result<Option<LitStr>> {
    let nexus_attrs: Vec<&Attribute> = attrs
        .iter()
        .filter(|attr| path_ends_with_ident(attr.path(), attr_name))
        .collect();
    if nexus_attrs.len() > 1 {
        return Err(Error::new_spanned(
            nexus_attrs[1],
            format!("duplicate `#[{attr_name}]` attribute"),
        ));
    }
    match nexus_attrs.first() {
        Some(attr) => {
            if attr.meta.require_list().is_err() && !matches!(attr.meta, syn::Meta::Path(_)) {
                return Err(Error::new_spanned(attr, "malformed attribute"));
            }
            parse_name_override(
                attr.meta
                    .require_list()
                    .map(|list| list.tokens.clone())
                    .unwrap_or_default(),
            )
        }
        None => Ok(None),
    }
}

pub(crate) fn trait_path_to_helper_module(service: &Path) -> Result<Path> {
    let mut helper = service.clone();
    let last = helper
        .segments
        .last_mut()
        .ok_or_else(|| Error::new_spanned(service, "expected a service trait path"))?;
    if !matches!(last.arguments, syn::PathArguments::None) {
        return Err(Error::new_spanned(
            &last.arguments,
            "service paths cannot include generic arguments",
        ));
    }
    let helper_ident = Ident::new(
        &format!("__nexus_service_metadata_for_{}", last.ident.unraw()),
        last.ident.span(),
    );
    last.ident = helper_ident;
    Ok(helper)
}

pub(crate) fn parse_methods_from_trait(item: &ItemTrait) -> Result<Vec<TraitItemFn>> {
    item.items
        .iter()
        .map(|item| match item {
            TraitItem::Fn(method) => Ok(method.clone()),
            other => Err(Error::new_spanned(
                other,
                "nexus services only support operation method declarations",
            )),
        })
        .collect()
}

pub(crate) fn error_tokens(error: Error) -> TokenStream {
    error.to_compile_error()
}
