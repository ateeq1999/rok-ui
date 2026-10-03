//! [`Suspense`] and [`ErrorBoundary`]: fallbacks for loading and errors.

use std::fmt;

use gpui::{div, prelude::*, AnyElement, App, Window};

use super::QueryError;
use crate::{theme::ActiveTheme, Cx};

/// Why [`Suspense`] content could not render yet.
#[derive(Clone, Debug)]
pub enum Suspend {
    /// Data is still loading: show the fallback.
    Pending,
    /// Loading failed: show the error view.
    Failed(QueryError),
}

impl From<QueryError> for Suspend {
    fn from(error: QueryError) -> Self {
        Self::Failed(error)
    }
}

impl<E: std::error::Error + Send + Sync + 'static> From<E> for Suspend {
    fn from(error: E) -> Self {
        Self::Failed(error.into())
    }
}

type Content = Box<dyn FnOnce(&mut Cx) -> Result<AnyElement, Suspend>>;
type ErrorView = Box<dyn FnOnce(&QueryError, &mut Cx) -> AnyElement>;

/// Renders content that reads queries, with a fallback while they load and an error view if
/// they fail. Content returns `Err` through `?` on
/// [`use_suspense_query`](super::use_suspense_query), so it never needs a loading branch:
///
/// ```no_run
/// # use rok_ui::{prelude::*, query::{self, QueryOptions, Suspense}, query_key};
/// # fn note_query() -> QueryOptions<String> {
/// #     QueryOptions::new(query_key!["note"], |_| async { Ok::<_, std::io::Error>(String::new()) })
/// # }
/// #[component]
/// fn NotePanel(cx: &mut Cx) -> impl IntoElement {
///     Suspense::new(|cx| {
///         let note = query::use_suspense_query(cx, note_query())?;
///         Ok(div().child(note.to_string()))
///     })
///     .fallback(Skeleton::new("note").h(px(120.)))
///     .error(|error, _| div().child(format!("Could not load the note: {error}")))
/// }
/// ```
///
/// Once data has loaded, refetches keep showing it: the fallback only covers the first load.
#[must_use = "components do nothing unless rendered as a child"]
pub struct Suspense {
    content: Content,
    fallback: Option<AnyElement>,
    error: Option<ErrorView>,
}

impl Suspense {
    /// Content that may suspend.
    pub fn new<E: IntoElement>(
        content: impl FnOnce(&mut Cx) -> Result<E, Suspend> + 'static,
    ) -> Self {
        Self {
            content: Box::new(move |cx| content(cx).map(IntoElement::into_any_element)),
            fallback: None,
            error: None,
        }
    }

    /// Shown while the content is pending. Default: nothing.
    pub fn fallback(mut self, fallback: impl IntoElement) -> Self {
        self.fallback = Some(fallback.into_any_element());
        self
    }

    /// Shown when the content failed. Default: the error message in the destructive color.
    pub fn error<E: IntoElement>(
        mut self,
        view: impl FnOnce(&QueryError, &mut Cx) -> E + 'static,
    ) -> Self {
        self.error = Some(Box::new(move |error, cx| {
            view(error, cx).into_any_element()
        }));
        self
    }
}

impl RenderOnce for Suspense {
    fn render(self, window: &mut Window, app: &mut App) -> impl IntoElement {
        let mut cx = Cx::new(window, app);
        match (self.content)(&mut cx) {
            Ok(content) => content,
            Err(Suspend::Pending) => self.fallback.unwrap_or_else(|| div().into_any_element()),
            Err(Suspend::Failed(error)) => match self.error {
                Some(view) => view(&error, &mut cx),
                None => default_error_view(&error, &cx),
            },
        }
    }
}

impl IntoElement for Suspense {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

fn default_error_view(error: &dyn fmt::Display, cx: &App) -> AnyElement {
    div()
        .text_color(cx.theme().colors.destructive)
        .child(crate::components::BidiText::new(error.to_string()))
        .into_any_element()
}

type Boundary = Box<dyn FnOnce(&mut Cx) -> AnyElement>;

/// Renders content that can fail, with a fallback for the error. A failure stays inside the
/// boundary: siblings render normally.
///
/// ```no_run
/// # use rok_ui::{prelude::*, query::ErrorBoundary};
/// #[component]
/// fn Settings(cx: &mut Cx) -> impl IntoElement {
///     ErrorBoundary::new(
///         |_cx| -> Result<_, std::num::ParseIntError> {
///             let port: u16 = "8080".parse()?;
///             Ok(div().child(format!("Port {port}")))
///         },
///         |error, _cx| div().child(format!("Invalid settings: {error}")),
///     )
/// }
/// ```
#[must_use = "components do nothing unless rendered as a child"]
pub struct ErrorBoundary {
    render: Boundary,
}

impl ErrorBoundary {
    /// Render `content`, or `fallback` with its error.
    pub fn new<V, F, E>(
        content: impl FnOnce(&mut Cx) -> Result<V, E> + 'static,
        fallback: impl FnOnce(E, &mut Cx) -> F + 'static,
    ) -> Self
    where
        V: IntoElement,
        F: IntoElement,
    {
        Self {
            render: Box::new(move |cx| match content(cx) {
                Ok(view) => view.into_any_element(),
                Err(error) => fallback(error, cx).into_any_element(),
            }),
        }
    }
}

impl RenderOnce for ErrorBoundary {
    fn render(self, window: &mut Window, app: &mut App) -> impl IntoElement {
        (self.render)(&mut Cx::new(window, app))
    }
}

impl IntoElement for ErrorBoundary {
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}
