use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Error, Expr, FnArg, Ident, ItemTrait, LitStr, MetaNameValue, Path, Result,
    ReturnType, TraitItem, TraitItemFn, Type,
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

pub(crate) fn parse_name_override(tokens: TokenStream) -> Result<Option<LitStr>> {
    if tokens.is_empty() {
        return Ok(None);
    }
    syn::parse2::<NameArg>(tokens).map(|parsed| Some(parsed.name))
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
