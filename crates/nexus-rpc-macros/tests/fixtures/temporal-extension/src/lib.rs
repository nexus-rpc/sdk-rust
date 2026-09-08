use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    FnArg, GenericArgument, ImplItemFn, PathArguments, ReturnType, Type, TypePath,
    parse_macro_input,
};

#[proc_macro_attribute]
pub fn temporal_operation(_args: TokenStream, item: TokenStream) -> TokenStream {
    let method = parse_macro_input!(item as ImplItemFn);
    match expand(method) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

fn expand(method: ImplItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let Some(asyncness) = &method.sig.asyncness else {
        return Err(syn::Error::new_spanned(
            &method.sig.ident,
            "fixture temporal operations must be async",
        ));
    };
    if method.sig.inputs.len() != 4 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations must take `&self`, context, client, and one input",
        ));
    }

    let Some(receiver) = method.sig.inputs.first().cloned() else {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations require `&self`",
        ));
    };
    if !matches!(receiver, FnArg::Receiver(_)) {
        return Err(syn::Error::new_spanned(
            receiver,
            "fixture temporal operations require `&self`",
        ));
    }

    let Some(context) = method.sig.inputs.iter().nth(1).cloned() else {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations require a typed context parameter",
        ));
    };
    let FnArg::Typed(context) = context else {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations require a typed context parameter",
        ));
    };
    let context_pat = context.pat.clone();
    let context_ty = context.ty.clone();

    let Some(client) = method.sig.inputs.iter().nth(2).cloned() else {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations require a typed client parameter",
        ));
    };
    let FnArg::Typed(client) = client else {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations require a typed client parameter",
        ));
    };
    let client_pat = client.pat.clone();
    let client_ty = client.ty.clone();

    let Some(input) = method.sig.inputs.iter().nth(3).cloned() else {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations require a typed input parameter",
        ));
    };
    let FnArg::Typed(input) = input else {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "fixture temporal operations require a typed input parameter",
        ));
    };
    let input_pat = input.pat.clone();
    let input_ty = input.ty.clone();

    let ReturnType::Type(_, output_ty) = &method.sig.output else {
        return Err(syn::Error::new_spanned(
            &method.sig.output,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    };
    let output_inner_ty = extract_operation_output_type(output_ty.as_ref())?;

    let attrs = &method.attrs;
    let vis = &method.vis;
    let name = &method.sig.ident;
    let helper_name = format_ident!("__temporal_operation_impl_{}", name);
    let body = &method.block;

    Ok(quote! {
        #asyncness fn #helper_name(
            &self,
            #context_pat: #context_ty,
            #client_pat: #client_ty,
            #input_pat: #input_ty,
        ) -> #output_ty #body

        #( #attrs )*
        #vis fn #name(&self) -> impl ::nexus_rpc::OperationHandler<#input_ty, #output_inner_ty> + '_ {
            struct __TemporalOperation<Start, Cancel> {
                start: Start,
                cancel: Cancel,
            }

            impl<I, O, Start, StartFuture, Cancel, CancelFuture> ::nexus_rpc::OperationHandler<I, O>
                for __TemporalOperation<Start, Cancel>
            where
                I: Send,
                Start: Fn(::nexus_rpc::OperationStartContext, I) -> StartFuture + Send + Sync,
                StartFuture: ::core::future::Future<
                        Output = ::core::result::Result<
                            ::nexus_rpc::OperationStartResult<O>,
                            ::nexus_rpc::BoxError,
                        >,
                    > + Send,
                Cancel: Fn(::nexus_rpc::OperationCancelContext) -> CancelFuture + Send + Sync,
                CancelFuture: ::core::future::Future<
                        Output = ::core::result::Result<(), ::nexus_rpc::BoxError>,
                    > + Send,
            {
                async fn start(
                    &self,
                    context: ::nexus_rpc::OperationStartContext,
                    input: I,
                ) -> ::core::result::Result<
                    ::nexus_rpc::OperationStartResult<O>,
                    ::nexus_rpc::BoxError,
                > {
                    (self.start)(context, input).await
                }

                async fn cancel(
                    &self,
                    context: ::nexus_rpc::OperationCancelContext,
                ) -> ::core::result::Result<(), ::nexus_rpc::BoxError> {
                    (self.cancel)(context).await
                }
            }

            __TemporalOperation {
                start: move |_context, input: #input_ty| async move {
                    let context = #context_ty::default();
                    let client = #client_ty::default();
                    self.#helper_name(context, client, input).await
                },
                cancel: move |_context| async move { Ok(()) },
            }
        }
    })
}

fn extract_operation_output_type(output_ty: &Type) -> syn::Result<Type> {
    let Type::Path(TypePath { path, .. }) = output_ty else {
        return Err(syn::Error::new_spanned(
            output_ty,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    };
    let result_segment = path.segments.last().ok_or_else(|| {
        syn::Error::new_spanned(
            output_ty,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        )
    })?;
    if result_segment.ident != "Result" {
        return Err(syn::Error::new_spanned(
            output_ty,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    }
    let PathArguments::AngleBracketed(arguments) = &result_segment.arguments else {
        return Err(syn::Error::new_spanned(
            &result_segment.arguments,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    };
    let Some(GenericArgument::Type(start_result_ty)) = arguments.args.first() else {
        return Err(syn::Error::new_spanned(
            arguments,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    };
    let Type::Path(TypePath {
        path: start_result_path,
        ..
    }) = start_result_ty
    else {
        return Err(syn::Error::new_spanned(
            start_result_ty,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    };
    let start_result_segment = start_result_path.segments.last().ok_or_else(|| {
        syn::Error::new_spanned(
            start_result_ty,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        )
    })?;
    if start_result_segment.ident != "OperationStartResult" {
        return Err(syn::Error::new_spanned(
            start_result_ty,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    }
    let PathArguments::AngleBracketed(output_arguments) = &start_result_segment.arguments else {
        return Err(syn::Error::new_spanned(
            &start_result_segment.arguments,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    };
    let Some(GenericArgument::Type(output_ty)) = output_arguments.args.first() else {
        return Err(syn::Error::new_spanned(
            output_arguments,
            "fixture temporal operations must return `Result<OperationStartResult<O>, BoxError>`",
        ));
    };
    Ok(output_ty.clone())
}
