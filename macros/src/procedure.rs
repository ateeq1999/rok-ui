//! `#[procedure]` and `#[memoize]`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, FnArg, GenericArgument, ItemFn, Pat, PathArguments, ReturnType, Token, Type,
};

/// `invalidates = [expr, ...]`.
struct ProcedureArguments {
    invalidates: Vec<Expr>,
}

impl Parse for ProcedureArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut invalidates = Vec::new();
        while !input.is_empty() {
            let name: syn::Ident = input.parse()?;
            if name != "invalidates" {
                return Err(syn::Error::new(
                    name.span(),
                    "expected `invalidates = [..]`",
                ));
            }
            input.parse::<Token![=]>()?;
            let list;
            syn::bracketed!(list in input);
            let keys = Punctuated::<Expr, Token![,]>::parse_terminated(&list)?;
            invalidates.extend(keys);
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { invalidates })
    }
}

fn is_task_cx(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .is_some_and(|segment| segment.ident == "TaskCx"),
        _ => false,
    }
}

/// `Result<O, E>` -> `(O, E)`.
fn result_types(output: &ReturnType) -> syn::Result<(Type, Type)> {
    let error = || syn::Error::new(output.span(), "a procedure returns `Result<Output, Error>`");
    let ReturnType::Type(_, ty) = output else {
        return Err(error());
    };
    let Type::Path(path) = ty.as_ref() else {
        return Err(error());
    };
    let segment = path.path.segments.last().ok_or_else(error)?;
    if segment.ident != "Result" {
        return Err(error());
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err(error());
    };
    let types: Vec<&Type> = arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        })
        .collect();
    match types.as_slice() {
        [output, error] => Ok(((*output).clone(), (*error).clone())),
        _ => Err(error()),
    }
}

pub fn expand_procedure(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let ProcedureArguments { invalidates } = syn::parse2(arguments)?;
    let function: ItemFn = syn::parse2(item)?;
    if function.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            function.sig.fn_token.span(),
            "#[procedure] functions are `async fn`",
        ));
    }
    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            function.sig.generics.span(),
            "#[procedure] functions cannot be generic",
        ));
    }
    let (output, error) = result_types(&function.sig.output)?;

    let mut context = None;
    let mut input = None;
    for argument in &function.sig.inputs {
        let FnArg::Typed(typed) = argument else {
            return Err(syn::Error::new(
                argument.span(),
                "procedures cannot take `self`",
            ));
        };
        if context.is_none() && input.is_none() && is_task_cx(&typed.ty) {
            context = Some(typed.pat.clone());
        } else if input.is_none() {
            input = Some((typed.pat.clone(), (*typed.ty).clone()));
        } else {
            return Err(syn::Error::new(
                typed.span(),
                "a procedure takes an optional `TaskCx` and one input; group inputs in a struct",
            ));
        }
    }

    let name = &function.sig.ident;
    let visibility = &function.vis;
    let body = &function.block;
    let docs: Vec<_> = function
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("doc"))
        .collect();
    let fallback_doc = format!("The `{name}` procedure.");
    let docs = if docs.is_empty() {
        quote!(#[doc = #fallback_doc])
    } else {
        quote!(#(#docs)*)
    };
    let bind_context = context.map_or_else(
        || quote!(let _ = __rok_cx;),
        |pattern| quote!(let #pattern = __rok_cx;),
    );
    let (input_type, bind_input) = match input {
        Some((pattern, ty)) => (quote!(#ty), quote!(let #pattern = __rok_input;)),
        None => (quote!(()), quote!(let () = __rok_input;)),
    };

    Ok(quote! {
        #docs
        #[allow(non_camel_case_types)]
        #[derive(Clone, Copy, Debug, Default)]
        #visibility struct #name;

        impl ::rok_ui::query::Procedure for #name {
            type Input = #input_type;
            type Output = #output;
            type Error = #error;

            fn run(
                &self,
                __rok_cx: ::rok_ui::query::TaskCx,
                __rok_input: Self::Input,
            ) -> ::rok_ui::query::BoxFuture<::core::result::Result<Self::Output, Self::Error>> {
                ::std::boxed::Box::pin(async move {
                    #bind_context
                    #bind_input
                    #body
                })
            }

            fn invalidates(&self) -> ::std::vec::Vec<::rok_ui::query::QueryKey> {
                ::std::vec![#(#invalidates),*]
            }
        }
    })
}

pub fn expand_memoize(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !arguments.is_empty() {
        return Err(syn::Error::new(
            arguments.span(),
            "#[memoize] takes no arguments",
        ));
    }
    let function: ItemFn = syn::parse2(item)?;
    if function.sig.asyncness.is_none() {
        return Err(syn::Error::new(
            function.sig.fn_token.span(),
            "#[memoize] functions are `async fn`",
        ));
    }
    let mut names = Vec::new();
    for argument in &function.sig.inputs {
        let FnArg::Typed(typed) = argument else {
            return Err(syn::Error::new(
                argument.span(),
                "#[memoize] functions cannot take `self`",
            ));
        };
        let Pat::Ident(pattern) = typed.pat.as_ref() else {
            return Err(syn::Error::new(
                typed.pat.span(),
                "parameters must be plain names",
            ));
        };
        if matches!(typed.ty.as_ref(), Type::Reference(_)) {
            return Err(syn::Error::new(
                typed.ty.span(),
                "memoized functions take owned arguments (the work outlives the call)",
            ));
        }
        names.push(pattern.ident.clone());
    }
    let output = match &function.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => quote!(#ty),
    };
    let mut signature = function.sig.clone();
    signature.asyncness = None;
    signature.output = syn::parse_quote!(-> ::rok_ui::query::memo::MemoFuture<#output>);
    let name = &function.sig.ident;
    let name_text = name.to_string();
    let attributes = &function.attrs;
    let visibility = &function.vis;
    let body = &function.block;
    let key = format_ident!("__rok_memo_key");
    Ok(quote! {
        #(#attributes)*
        #visibility #signature {
            let #key = ::rok_ui::query::memo::key(::core::module_path!(), #name_text, &(#(&#names,)*));
            ::rok_ui::query::memo::memoize(#key, move || async move #body)
        }
    })
}
