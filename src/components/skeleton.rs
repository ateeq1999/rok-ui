//! Skeleton: a pulsing placeholder while content loads.

use std::time::Duration;

use gpui::{
    div, prelude::*, pulsating_between, Animation, AnimationExt, App, ElementId, StyleRefinement,
    Window,
};

use crate::sx::SxStyled;
use crate::{styles, styles::ApplyStyleOverrides};

styles! {
    SKELETON = { root: { radius: md, background: muted } }
}

/// Size it like the content it stands in for: `Skeleton::new("title").h(px(16.)).w(px(200.))`.
#[derive(IntoElement)]
pub struct Skeleton {
    id: ElementId,
    sx: crate::sx::Sx,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Skeleton);

impl Skeleton {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            sx: crate::sx::Sx::new(),
            style_overrides: StyleRefinement::default(),
        }
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .sx((&SKELETON.root, &self.sx))
            .apply_style_overrides(&self.style_overrides)
            .with_animation(
                self.id,
                Animation::new(Duration::from_millis(2000))
                    .repeat()
                    .with_easing(pulsating_between(0.5, 1.0)),
                |skeleton, opacity| skeleton.opacity(opacity),
            )
    }
}
