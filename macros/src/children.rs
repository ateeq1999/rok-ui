//! `children![..]` and `view! { .. }`: element trees with control flow,
//! compiled to plain GPUI builder calls.

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    braced, bracketed,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    token, Expr, Ident, Pat, Path, Token,
};

/// A node in a children list.
enum Node {
    /// Any expression implementing `IntoElement`.
    Element(TokenStream),
    If {
        condition: TokenStream,
        then_branch: Vec<Node>,
        else_branch: Option<Vec<Node>>,
    },
    For {
        pattern: Pat,
        iterable: Expr,
        body: Vec<Node>,
    },
    Match {
        scrutinee: Expr,
        arms: Vec<MatchArm>,
    },
}

struct MatchArm {
    pattern: Pat,
    guard: Option<Expr>,
    body: Vec<Node>,
}

/// The two surface syntaxes share control flow but differ in how plain
/// children are written and separated.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Syntax {
    /// `children![a, if c { b } else { d }, for x in xs => e(x)]`
    List,
    /// `view! { Card { Title("x") if c { Badge("y") } } }`
    Markup,
}

fn parse_nodes(input: ParseStream, syntax: Syntax) -> syn::Result<Vec<Node>> {
    let mut nodes = Vec::new();
    while !input.is_empty() {
        nodes.push(parse_node(input, syntax)?);
        if syntax == Syntax::List {
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        } else if input.peek(Token![,]) {
            // Commas between markup nodes are allowed but not required.
            input.parse::<Token![,]>()?;
        }
    }
    Ok(nodes)
}

fn parse_braced_nodes(input: ParseStream, syntax: Syntax) -> syn::Result<Vec<Node>> {
    let content;
    braced!(content in input);
    parse_nodes(&content, syntax)
}

fn parse_node(input: ParseStream, syntax: Syntax) -> syn::Result<Node> {
    if input.peek(Token![if]) {
        return parse_if(input, syntax);
    }
    if input.peek(Token![for]) {
        input.parse::<Token![for]>()?;
        let pattern = Pat::parse_multi_with_leading_vert(input)?;
        input.parse::<Token![in]>()?;
        let iterable = Expr::parse_without_eager_brace(input)?;
        let body = if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;
            vec![parse_single(input, syntax)?]
        } else {
            parse_braced_nodes(input, syntax)?
        };
        return Ok(Node::For {
            pattern,
            iterable,
            body,
        });
    }
    if input.peek(Token![match]) {
        input.parse::<Token![match]>()?;
        let scrutinee = Expr::parse_without_eager_brace(input)?;
        let content;
        braced!(content in input);
        let mut arms = Vec::new();
        while !content.is_empty() {
            let pattern = Pat::parse_multi_with_leading_vert(&content)?;
            let guard = if content.peek(Token![if]) {
                content.parse::<Token![if]>()?;
                Some(content.parse::<Expr>()?)
            } else {
                None
            };
            content.parse::<Token![=>]>()?;
            let body = if content.peek(token::Brace) {
                parse_braced_nodes(&content, syntax)?
            } else {
                vec![parse_single(&content, syntax)?]
            };
            arms.push(MatchArm {
                pattern,
                guard,
                body,
            });
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        return Ok(Node::Match { scrutinee, arms });
    }
    parse_single(input, syntax)
}

fn parse_if(input: ParseStream, syntax: Syntax) -> syn::Result<Node> {
    input.parse::<Token![if]>()?;
    let condition = if input.peek(Token![let]) {
        input.parse::<Token![let]>()?;
        let pattern = Pat::parse_multi_with_leading_vert(input)?;
        input.parse::<Token![=]>()?;
        let value = Expr::parse_without_eager_brace(input)?;
        quote!(let #pattern = #value)
    } else {
        Expr::parse_without_eager_brace(input)?.into_token_stream()
    };
    let then_branch = parse_braced_nodes(input, syntax)?;
    let else_branch = if input.peek(Token![else]) {
        input.parse::<Token![else]>()?;
        if input.peek(Token![if]) {
            Some(vec![parse_if(input, syntax)?])
        } else {
            Some(parse_braced_nodes(input, syntax)?)
        }
    } else {
        None
    };
    Ok(Node::If {
        condition,
        then_branch,
        else_branch,
    })
}

/// A plain child: an expression in list syntax, a markup element otherwise.
fn parse_single(input: ParseStream, syntax: Syntax) -> syn::Result<Node> {
    match syntax {
        Syntax::List => {
            let expression: Expr = input.parse()?;
            if let Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(text),
                ..
            }) = &expression
            {
                return Ok(Node::Element(text_child(text)));
            }
            Ok(Node::Element(expression.into_token_stream()))
        }
        Syntax::Markup => parse_markup_element(input),
    }
}

/// A string literal child. Text with right-to-left letters becomes a `BidiText`,
/// so it displays in the right order on every platform; other text stays a plain
/// string.
fn text_child(text: &syn::LitStr) -> TokenStream {
    let has_rtl = text.value().chars().any(|character| {
        matches!(character as u32,
            0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF | 0x10800..=0x10FFF
            | 0x1E800..=0x1EFFF)
    });
    if has_rtl {
        quote::quote!(::rok_ui::components::BidiText::new(#text))
    } else {
        text.into_token_stream()
    }
}

/// One markup child:
/// - `"text"`: a string literal,
/// - `{ expr }`: any `IntoElement` expression,
/// - `div(args) { children }` / `Component(args).method(..) { children }`.
fn parse_markup_element(input: ParseStream) -> syn::Result<Node> {
    if input.peek(syn::LitStr) {
        let text: syn::LitStr = input.parse()?;
        return Ok(Node::Element(text_child(&text)));
    }
    if input.peek(token::Brace) {
        let content;
        braced!(content in input);
        let expression: Expr = content.parse()?;
        return Ok(Node::Element(expression.into_token_stream()));
    }

    let path: Path = input.call(Path::parse_mod_style)?;
    let arguments = if input.peek(token::Paren) {
        let content;
        parenthesized!(content in input);
        parse_arguments(&content)?
    } else {
        Arguments::default()
    };

    // `.method(..)` calls passed through as written.
    let mut methods = TokenStream::new();
    while input.peek(Token![.]) {
        let dot: Token![.] = input.parse()?;
        let method = input.call(Ident::parse_any)?;
        let content;
        let parentheses = parenthesized!(content in input);
        let method_arguments: TokenStream = content.parse()?;
        dot.to_tokens(&mut methods);
        method.to_tokens(&mut methods);
        parentheses.surround(&mut methods, |tokens| tokens.extend(method_arguments));
    }

    let children = if input.peek(token::Brace) {
        Some(parse_braced_nodes(input, Syntax::Markup)?)
    } else {
        None
    };

    // Lowercase single-segment names are GPUI element functions: `div()`, `img(src)`.
    let is_element_function = path.segments.len() == 1
        && path
            .segments
            .first()
            .map(|segment| {
                segment
                    .ident
                    .to_string()
                    .starts_with(|character: char| character.is_ascii_lowercase())
            })
            .unwrap_or(false);
    let positional = &arguments.positional;
    let mut expression = if is_element_function {
        let function = &path.segments[0].ident;
        if matches!(
            function.to_string().as_str(),
            "div" | "svg" | "img" | "canvas"
        ) {
            quote!(::rok_ui::gpui::#function(#(#positional),*))
        } else {
            quote!(#function(#(#positional),*))
        }
    } else {
        quote!(#path::new(#(#positional),*))
    };
    for (name, value) in &arguments.named {
        expression = match name.to_string().as_str() {
            "sx" => quote!(::rok_ui::sx::SxStyled::sx(#expression, &(#value))),
            _ => quote!(#expression.#name(#value)),
        };
    }
    expression = quote!(#expression #methods);
    if let Some(children) = children {
        let children = generate_children(&children);
        expression = quote!(::rok_ui::gpui::ParentElement::children(#expression, #children));
    }
    Ok(Node::Element(expression))
}

#[derive(Default)]
struct Arguments {
    positional: Vec<Expr>,
    named: Vec<(Ident, TokenStream)>,
}

/// `(positional, ..., name = value, sx = [a, cond => b])`.
fn parse_arguments(input: ParseStream) -> syn::Result<Arguments> {
    let mut arguments = Arguments::default();
    while !input.is_empty() {
        let is_named =
            input.peek(Ident::peek_any) && input.peek2(Token![=]) && !input.peek2(Token![==]);
        if is_named {
            let name = input.call(Ident::parse_any)?;
            input.parse::<Token![=]>()?;
            let value = if name == "sx" && input.peek(token::Bracket) {
                // `sx = [base, cond => extra]` merges like `sx![..]`.
                let content;
                bracketed!(content in input);
                let list: TokenStream = content.parse()?;
                quote!(::rok_ui::sx![#list])
            } else {
                input.parse::<Expr>()?.into_token_stream()
            };
            arguments.named.push((name, value));
        } else {
            arguments.positional.push(input.parse()?);
        }
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(arguments)
}

/// Statements that push `nodes` onto the vector named `target`.
fn generate_pushes(nodes: &[Node], target: &Ident) -> TokenStream {
    let statements = nodes.iter().map(|node| match node {
        Node::Element(expression) => quote! {
            #target.push(::rok_ui::gpui::IntoElement::into_any_element(#expression));
        },
        Node::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let then_pushes = generate_pushes(then_branch, target);
            let else_pushes = else_branch
                .as_ref()
                .map(|branch| {
                    let pushes = generate_pushes(branch, target);
                    quote!(else { #pushes })
                })
                .unwrap_or_default();
            quote!(if #condition { #then_pushes } #else_pushes)
        }
        Node::For {
            pattern,
            iterable,
            body,
        } => {
            let pushes = generate_pushes(body, target);
            quote!(for #pattern in #iterable { #pushes })
        }
        Node::Match { scrutinee, arms } => {
            let arms = arms.iter().map(|arm| {
                let MatchArm {
                    pattern,
                    guard,
                    body,
                } = arm;
                let guard = guard.as_ref().map(|guard| quote!(if #guard));
                let pushes = generate_pushes(body, target);
                quote!(#pattern #guard => { #pushes })
            });
            quote!(match #scrutinee { #(#arms)* })
        }
    });
    quote!(#(#statements)*)
}

/// A `Vec<AnyElement>` expression holding `nodes`.
fn generate_children(nodes: &[Node]) -> TokenStream {
    let target = Ident::new("__rok_ui_children", Span::mixed_site());
    let pushes = generate_pushes(nodes, &target);
    quote! {{
        #[allow(unused_mut)]
        let mut #target: ::std::vec::Vec<::rok_ui::gpui::AnyElement> = ::std::vec::Vec::new();
        #pushes
        #target
    }}
}

struct ChildrenInput(Vec<Node>);

impl Parse for ChildrenInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ChildrenInput(parse_nodes(input, Syntax::List)?))
    }
}

struct ViewInput(Vec<Node>);

impl Parse for ViewInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(ViewInput(parse_nodes(input, Syntax::Markup)?))
    }
}

pub fn expand_children(input: TokenStream) -> syn::Result<TokenStream> {
    let ChildrenInput(nodes) = syn::parse2(input)?;
    Ok(generate_children(&nodes))
}

/// One root element becomes that element; several roots (or control flow at
/// the root) become a `Vec<AnyElement>`.
pub fn expand_view(input: TokenStream) -> syn::Result<TokenStream> {
    let ViewInput(nodes) = syn::parse2(input)?;
    match nodes.as_slice() {
        [Node::Element(expression)] => Ok(expression.clone()),
        _ => Ok(generate_children(&nodes)),
    }
}
