//! `file_route! { .. }`: one route file's declaration, read by `rok-ui-build` too.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Expr, Ident, Path, Token, Type,
};

/// `name: Type` inside `params: { .. }`.
struct Param {
    _name: Ident,
    _colon: Token![:],
    _ty: Type,
}

impl Parse for Param {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _name: input.parse()?,
            _colon: input.parse()?,
            _ty: input.parse()?,
        })
    }
}

#[derive(Default)]
struct FileRoute {
    search: Option<Type>,
    component: Option<Path>,
    layout: Option<Path>,
    before_load: Option<Expr>,
    loader: Option<Expr>,
}

impl Parse for FileRoute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut route = FileRoute::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            match key.to_string().as_str() {
                "params" => {
                    let content;
                    syn::braced!(content in input);
                    // Types are read by the route generator; here they only need to parse.
                    Punctuated::<Param, Token![,]>::parse_terminated(&content)?;
                }
                "search" => route.search = Some(input.parse()?),
                "component" => route.component = Some(input.parse()?),
                "layout" => route.layout = Some(input.parse()?),
                "before_load" => route.before_load = Some(input.parse()?),
                "loader" => route.loader = Some(input.parse()?),
                _ => return Err(syn::Error::new(
                    key.span(),
                    "expected `params`, `search`, `component`, `layout`, `before_load` or `loader`",
                )),
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        if route.loader.is_some() && route.layout.is_some() {
            return Err(input.error("`loader` belongs on pages (`component: ..`), not layouts"));
        }
        if route.component.is_some() == route.layout.is_some() {
            return Err(input.error(
                "a route file declares exactly one of `component: Page` or `layout: Layout`",
            ));
        }
        Ok(route)
    }
}

pub fn expand_file_route(input: TokenStream) -> syn::Result<TokenStream> {
    let route: FileRoute = syn::parse2(input)?;
    let gpui = quote!(::rok_ui::gpui);
    let router = quote!(::rok_ui::router);
    let search = route.search.map(|search| {
        quote! {
            /// This route's search params.
            pub type RouteSearch = #search;

            /// The current search params of this route.
            pub fn search(cx: &mut #gpui::App) -> RouteSearch {
                #router::use_search::<RouteSearch>(cx)
            }
        }
    });
    let guard = route.before_load.map_or_else(
        || quote!(::core::result::Result::Ok(())),
        |before_load| {
            quote! {
                let guard: fn(&#router::Location, &mut #gpui::App) -> ::core::result::Result<(), #router::RouteControl> = #before_load;
                guard(location, cx)
            }
        },
    );
    let loader = route.loader.map_or_else(
        || quote!(let _ = (route, cx);),
        |loader| {
            quote! {
                if let ::core::option::Option::Some(typed) = <Route as #router::Route>::from_match(route) {
                    let loader: fn(&Route, &mut #gpui::App) = #loader;
                    loader(&typed, cx);
                }
            }
        },
    );
    let render = match (route.component, route.layout) {
        (Some(component), _) => quote! {
            #[doc(hidden)]
            pub fn __rok_loader(route: &#router::RouteMatch, cx: &mut #gpui::App) {
                #loader
            }

            #[doc(hidden)]
            pub fn __rok_page(
                _route: &#router::RouteMatch,
                _window: &mut #gpui::Window,
                _cx: &mut #gpui::App,
            ) -> ::core::option::Option<#gpui::AnyElement> {
                ::core::option::Option::Some(#gpui::IntoElement::into_any_element(#component::new()))
            }
        },
        (None, Some(layout)) => quote! {
            #[doc(hidden)]
            pub fn __rok_layout(
                outlet: #gpui::AnyElement,
                _route: &#router::RouteMatch,
                _window: &mut #gpui::Window,
                _cx: &mut #gpui::App,
            ) -> #gpui::AnyElement {
                #gpui::IntoElement::into_any_element(#gpui::ParentElement::child(#layout::new(), outlet))
            }
        },
        (None, None) => unreachable!("checked while parsing"),
    };
    Ok(quote! {
        #search

        #[doc(hidden)]
        #[allow(unused_variables)]
        pub fn __rok_guard(
            location: &#router::Location,
            cx: &mut #gpui::App,
        ) -> ::core::result::Result<(), #router::RouteControl> {
            #guard
        }

        #render
    })
}
