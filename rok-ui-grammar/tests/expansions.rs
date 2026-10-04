//! Expansions are valid Rust, deterministic, and errors point at the mistake.

use proc_macro2::TokenStream;
use quote::quote;
use rok_ui_grammar::{
    children, component, file_route, form_values, procedure, search, store, styles,
};

/// The expansion as a parsed file (proving it is valid Rust) and as text.
fn expand(result: syn::Result<TokenStream>) -> (syn::File, String) {
    let tokens = result.unwrap_or_else(|error| panic!("expansion failed: {error}"));
    let text = tokens.to_string();
    let file = syn::parse2::<syn::File>(tokens)
        .unwrap_or_else(|error| panic!("not valid items: {error}\n{text}"));
    (file, text)
}

fn error(result: syn::Result<TokenStream>) -> String {
    result.expect_err("an error").to_string()
}

fn item_names(file: &syn::File) -> Vec<String> {
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) => Some(item.ident.to_string()),
            syn::Item::Static(item) => Some(item.ident.to_string()),
            _ => None,
        })
        .collect()
}

#[test]
fn components_get_a_struct_a_constructor_and_builders() {
    let input = quote! {
        /// A greeting.
        pub fn Greeting(
            name: SharedString,
            #[default] excited: bool,
            #[default(px(32.))] size: Pixels,
            #[children] children: Vec<AnyElement>,
            cx: &mut Cx,
        ) -> impl IntoElement { div() }
    };
    let (file, text) = expand(component::expand(TokenStream::new(), input.clone()));
    assert!(item_names(&file).contains(&"Greeting".to_string()));
    assert!(text.contains("pub fn new (name : impl :: core :: convert :: Into < SharedString >)"));
    assert!(text.contains("pub fn excited"));
    assert!(text.contains("let value : Pixels = px (32.) ;"));
    assert!(text.contains("ParentElement for Greeting"));
    assert!(
        text.contains(":: rok_ui :: Cx :: new"),
        "a `cx: &mut Cx` component builds one"
    );
    assert_eq!(
        text,
        component::expand(TokenStream::new(), input)
            .unwrap()
            .to_string(),
        "deterministic"
    );
}

#[test]
fn component_mistakes_are_errors() {
    assert!(error(component::expand(
        quote!(x),
        quote!(
            fn A() -> impl IntoElement {
                div()
            }
        )
    ))
    .contains("takes no arguments"));
    assert!(error(component::expand(
        TokenStream::new(),
        quote!(
            fn A<T>() -> impl IntoElement {
                div()
            }
        )
    ))
    .contains("cannot be generic"));
    assert!(error(component::expand(
        TokenStream::new(),
        quote!(
            fn A(#[wat] x: u8) -> impl IntoElement {
                div()
            }
        )
    ))
    .contains("unsupported attribute"));
    assert!(error(component::expand(
        TokenStream::new(),
        quote!(
            fn A(window: &mut Window, cx: &mut Cx) -> impl IntoElement {
                div()
            }
        )
    ))
    .contains("remove the `window` parameter"));
}

#[test]
fn styles_become_statics_with_typed_fields() {
    let (file, text) = expand(styles::expand_styles(quote! {
        pub CARD = {
            base: { display: flex, padding: 6, background: card, hover: { border_color: ring } },
            tone(Tone): { Calm: { background: muted }, Loud: { background: primary/90 } },
        }
    }));
    let names = item_names(&file);
    assert!(names.contains(&"CardStyles".to_string()) && names.contains(&"CARD".to_string()));
    assert!(text.contains("pub fn tone (& self , value : Tone)"));
    assert!(error(styles::expand_styles(quote!(X = { a: { paddin: 1 } }))).contains("padding"));
}

#[test]
fn keyed_loops_wrap_items_in_keyed() {
    let tokens = children::expand_view(quote! {
        #[key(todo.id)]
        for todo in todos { Row(todo.title.clone()) }
    })
    .unwrap()
    .to_string();
    assert!(tokens.contains(":: rok_ui :: Keyed :: new"));
    assert!(error(children::expand_view(quote!(
        #[wat]
        for x in y {
            A()
        }
    )))
    .contains("key"));
}

#[test]
fn procedures_become_unit_structs() {
    let (file, text) = expand(procedure::expand_procedure(
        quote!(invalidates = [query_key!["notes"]]),
        quote! {
            pub async fn create_note(cx: TaskCx, input: NewNote) -> Result<Note, NoteError> {
                Ok(Note::from(input))
            }
        },
    ));
    assert!(item_names(&file).contains(&"create_note".to_string()));
    assert!(text.contains("type Input = NewNote"));
    assert!(text.contains("type Error = NoteError"));
    assert!(text.contains("query_key ! [\"notes\"]"));
    assert!(error(procedure::expand_procedure(
        TokenStream::new(),
        quote!(
            fn f(input: u8) -> Result<u8, E> {
                Ok(input)
            }
        )
    ))
    .contains("async fn"));
    assert!(error(procedure::expand_procedure(
        TokenStream::new(),
        quote!(
            async fn f(input: u8) -> u8 {
                input
            }
        )
    ))
    .contains("Result<Output, Error>"));
}

#[test]
fn memoized_functions_return_shared_futures() {
    let (_, text) = expand(procedure::expand_memoize(
        TokenStream::new(),
        quote!(
            async fn rate(currency: String) -> f64 {
                1.0
            }
        ),
    ));
    assert!(text.contains("MemoFuture < f64 >"));
    assert!(text.contains("MemoScope :: App"));
    let (_, navigation) = expand(procedure::expand_memoize(
        quote!(scope = navigation),
        quote!(
            async fn f() -> u8 {
                1
            }
        ),
    ));
    assert!(navigation.contains("MemoScope :: Navigation"));
    let (_, timed) = expand(procedure::expand_memoize(
        quote!(ttl_ms = 500),
        quote!(
            async fn f() -> u8 {
                1
            }
        ),
    ));
    assert!(timed.contains("from_millis (500u64)"), "{timed}");
    assert!(error(procedure::expand_memoize(
        quote!(scope = forever),
        quote!(
            async fn f() -> u8 {
                1
            }
        )
    ))
    .contains("scope = navigation"));
    assert!(error(procedure::expand_memoize(
        TokenStream::new(),
        quote!(
            async fn rate(currency: &str) -> f64 {
                1.0
            }
        )
    ))
    .contains("owned arguments"));
}

#[test]
fn derives_generate_their_impls() {
    let (_, search) = expand(search::expand_search(quote! {
        struct S { #[search(default = 1)] page: u32, q: Option<String>, #[search(rename = "t")] tab: Tab }
    }));
    assert!(search.contains("& [\"page\" , \"q\" , \"t\"]"), "{search}");

    let (_, values) = expand(form_values::expand_form_values(quote! {
        struct SignUp { pub email: String, team: Vec<Invite> }
    }));
    assert!(values.contains("pub const EMAIL : :: rok_ui :: form :: Field < Self , String >"));
    assert!(values.contains("const TEAM"));

    let (file, store) = expand(store::expand_store(
        quote! { pub struct Todo { title: String, done: bool } },
    ));
    assert!(item_names(&file).contains(&"TodoStore".to_string()));
    assert!(store.contains("pub fn set_done"));
    assert!(error(store::expand_store(quote!(
        struct T {
            get: u8,
        }
    )))
    .contains("clashes"));
}

#[test]
fn route_files_declare_one_page_or_layout() {
    let (_, page) = expand(file_route::expand_file_route(quote! {
        params: { id: u64 },
        search: NoteSearch,
        loader: |route, cx| prefetch(cx, route.id),
        component: NotePage,
    }));
    assert!(page.contains("pub fn __rok_page"));
    assert!(page.contains("pub type RouteSearch = NoteSearch"));
    assert!(page.contains("pub fn __rok_loader"));

    let (_, layout) = expand(file_route::expand_file_route(quote!(layout: Shell)));
    assert!(layout.contains("pub fn __rok_layout"));

    assert!(error(file_route::expand_file_route(
        quote!(component: A, layout: B)
    ))
    .contains("exactly one"));
    assert!(error(file_route::expand_file_route(
        quote!(loader: |r, cx| (), layout: B)
    ))
    .contains("belongs on pages"));
    assert!(error(file_route::expand_file_route(quote!(page: A))).contains("expected"));
}
