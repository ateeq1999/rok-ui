//! Questionnaire: a multi-step form of single-choice, multiple-choice and
//! freeform questions, any of which can be skippable.

use std::rc::Rc;

use gpui::{
    div, prelude::*, px, AnyElement, App, CursorStyle, ElementId, FontWeight, SharedString,
    StyleRefinement, Window,
};

use super::{
    button::Button,
    input::{use_textarea_state, Textarea},
    interaction::Callback,
    overlay::child_id,
    progress::Progress,
};
use crate::{
    hooks::{use_keyed_state, EventHandler, State},
    icon::{Icon, IconName},
    styles::ApplyStyleOverrides,
    theme::ActiveTheme,
};

#[derive(Clone)]
enum QuestionKind {
    Single(Vec<SharedString>),
    Multiple(Vec<SharedString>),
    Freeform,
}

/// One step of a [`Questionnaire`].
#[derive(Clone)]
pub struct Question {
    id: SharedString,
    title: SharedString,
    description: Option<SharedString>,
    kind: QuestionKind,
    skippable: bool,
    placeholder: SharedString,
}

impl Question {
    fn new(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        kind: QuestionKind,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: None,
            kind,
            skippable: false,
            placeholder: "Type your answer…".into(),
        }
    }

    /// Pick exactly one option.
    pub fn single(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self::new(
            id,
            title,
            QuestionKind::Single(options.into_iter().map(Into::into).collect()),
        )
    }

    /// Pick one or more options.
    pub fn multiple(
        id: impl Into<SharedString>,
        title: impl Into<SharedString>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        Self::new(
            id,
            title,
            QuestionKind::Multiple(options.into_iter().map(Into::into).collect()),
        )
    }

    /// Answer in your own words.
    pub fn freeform(id: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self::new(id, title, QuestionKind::Freeform)
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Offer a Skip button.
    pub fn skippable(mut self) -> Self {
        self.skippable = true;
        self
    }

    /// Placeholder of a freeform answer.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
}

/// The answer to one question.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    Choice(SharedString),
    Choices(Vec<SharedString>),
    Text(SharedString),
    Skipped,
}

impl Answer {
    /// A one-line summary for review screens.
    pub fn summary(&self) -> SharedString {
        match self {
            Answer::Choice(choice) => choice.clone(),
            Answer::Choices(choices) => choices
                .iter()
                .map(|choice| choice.as_ref())
                .collect::<Vec<_>>()
                .join(", ")
                .into(),
            Answer::Text(text) => text.clone(),
            Answer::Skipped => "Skipped".into(),
        }
    }
}

/// A question's id with its answer, as passed to `on_complete`.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestionnaireAnswer {
    pub question_id: SharedString,
    pub answer: Answer,
}

struct QuestionnaireMemory {
    step: usize,
    answers: Vec<Option<Answer>>,
    finished: bool,
}

/// Progress, answers and the current step are kept per id.
///
/// ```ignore
/// Questionnaire::new("onboarding")
///     .question(Question::single("role", "What best describes you?", ["Engineer", "Designer", "Founder"]))
///     .question(Question::multiple("tools", "Which tools do you use?", ["Figma", "VS Code", "Zed"]))
///     .question(Question::freeform("goal", "What do you want to build?").skippable())
///     .on_complete(|answers, _, cx| save_answers(answers, cx))
/// ```
#[derive(IntoElement)]
pub struct Questionnaire {
    id: ElementId,
    questions: Vec<Question>,
    on_complete: Option<EventHandler<Vec<QuestionnaireAnswer>>>,
    style_overrides: StyleRefinement,
}

crate::implement_style_overrides!(Questionnaire);

impl Questionnaire {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            questions: Vec::new(),
            on_complete: None,
            style_overrides: StyleRefinement::default(),
        }
    }

    pub fn question(mut self, question: Question) -> Self {
        self.questions.push(question);
        self
    }

    /// Called with every answer when the last question is answered or skipped.
    pub fn on_complete(
        mut self,
        handler: impl Fn(&Vec<QuestionnaireAnswer>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_complete = Some(Rc::new(handler));
        self
    }
}

/// Record `answer` for the current step and advance, finishing after the last one.
fn advance(
    memory: &State<QuestionnaireMemory>,
    answer: Answer,
    questions: &[Question],
    on_complete: Option<&EventHandler<Vec<QuestionnaireAnswer>>>,
    window: &mut Window,
    cx: &mut App,
) {
    memory.update(cx, |memory| {
        if let Some(slot) = memory.answers.get_mut(memory.step) {
            *slot = Some(answer);
        }
        if memory.step + 1 < questions.len() {
            memory.step += 1;
        } else {
            memory.finished = true;
        }
    });
    if memory.read(cx).finished {
        let answers: Vec<QuestionnaireAnswer> = questions
            .iter()
            .zip(memory.read(cx).answers.iter())
            .map(|(question, answer)| QuestionnaireAnswer {
                question_id: question.id.clone(),
                answer: answer.clone().unwrap_or(Answer::Skipped),
            })
            .collect();
        if let Some(handler) = on_complete {
            handler(&answers, window, cx);
        }
    }
}

impl RenderOnce for Questionnaire {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let question_count = self.questions.len();
        let memory: State<QuestionnaireMemory> =
            use_keyed_state(child_id(&self.id, "questionnaire"), window, cx, || {
                QuestionnaireMemory {
                    step: 0,
                    answers: vec![None; question_count],
                    finished: false,
                }
            });
        if memory.read(cx).answers.len() != question_count {
            memory.update(cx, |memory| {
                memory.answers.resize(question_count, None);
                memory.step = memory.step.min(question_count.saturating_sub(1));
            });
        }
        let theme = cx.theme();
        let colors = theme.colors.clone();
        let radius = theme.radius_medium();
        let ring_color = colors.ring;
        let questions: Rc<Vec<Question>> = Rc::new(self.questions);
        let on_complete = self.on_complete;
        let (step, finished) = {
            let memory = memory.read(cx);
            (memory.step, memory.finished)
        };

        let container = div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap(px(20.))
            .w_full()
            .max_w(px(560.))
            .p(px(24.))
            .rounded(theme.radius_extra_large())
            .border_1()
            .border_color(colors.border)
            .bg(colors.card)
            .text_color(colors.card_foreground);

        if finished || question_count == 0 {
            let answers = memory.read(cx).answers.clone();
            let restart_memory = memory.clone();
            let summary = questions.iter().zip(answers).map(|(question, answer)| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(question.title.clone()),
                    )
                    .child(
                        div()
                            .text_sm()
                            .child(answer.unwrap_or(Answer::Skipped).summary()),
                    )
            });
            return container
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.))
                        .child(Icon::new(IconName::CircleCheck).size(px(24.)))
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("All done"),
                        ),
                )
                .child(div().flex().flex_col().gap(px(12.)).children(summary))
                .child(
                    div().flex().justify_end().child(
                        Button::new("questionnaire-restart")
                            .outline()
                            .label("Start over")
                            .on_click(move |_, _, cx| {
                                restart_memory.update(cx, |memory| {
                                    memory.step = 0;
                                    memory.finished = false;
                                    memory.answers = vec![None; question_count];
                                })
                            }),
                    ),
                )
                .apply_style_overrides(&self.style_overrides);
        }

        let question = questions[step].clone();
        let current_answer = memory.read(cx).answers[step].clone();
        let freeform_key = ElementId::NamedChild(
            Box::new(child_id(&self.id, "freeform")),
            question.id.clone(),
        );
        let placeholder = question.placeholder.clone();
        let freeform = matches!(question.kind, QuestionKind::Freeform).then(|| {
            use_textarea_state(freeform_key, window, cx, |state| {
                state.with_placeholder(placeholder)
            })
        });

        let option_row = |index: usize,
                          label: SharedString,
                          selected: bool,
                          multiple: bool,
                          on_pick: Callback| {
            let indicator = div()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .size(px(16.))
                .border_1()
                .border_color(if selected {
                    colors.primary
                } else {
                    colors.input
                })
                .map(|indicator| {
                    if multiple {
                        indicator.rounded(px(4.)).when(selected, |indicator| {
                            indicator.bg(colors.primary).child(
                                Icon::new(IconName::Check)
                                    .size(px(12.))
                                    .color(colors.primary_foreground),
                            )
                        })
                    } else {
                        indicator.rounded_full().when(selected, |indicator| {
                            indicator.child(div().size(px(8.)).rounded_full().bg(colors.primary))
                        })
                    }
                });
            div()
                .id(("questionnaire-option", index))
                .flex()
                .items_center()
                .gap(px(12.))
                .px(px(14.))
                .py(px(10.))
                .rounded(radius)
                .border_1()
                .border_color(if selected {
                    colors.primary
                } else {
                    colors.border
                })
                .when(selected, |row| row.bg(colors.accent))
                .text_sm()
                .cursor(CursorStyle::PointingHand)
                .tab_index(0)
                .focus(move |style| style.border_color(ring_color))
                .hover(|style| style.bg(colors.accent))
                .on_click(move |_, window, cx| on_pick(window, cx))
                .child(indicator)
                .child(label)
                .into_any_element()
        };

        let body: AnyElement = match &question.kind {
            QuestionKind::Single(options) => div()
                .flex()
                .flex_col()
                .gap(px(8.))
                .children(options.iter().enumerate().map(|(index, option)| {
                    let selected = current_answer == Some(Answer::Choice(option.clone()));
                    let memory = memory.clone();
                    let option = option.clone();
                    option_row(
                        index,
                        option.clone(),
                        selected,
                        false,
                        Rc::new(move |_, cx| {
                            memory.update(cx, |memory| {
                                memory.answers[memory.step] = Some(Answer::Choice(option.clone()))
                            })
                        }),
                    )
                }))
                .into_any_element(),
            QuestionKind::Multiple(options) => {
                let chosen: Vec<SharedString> = match &current_answer {
                    Some(Answer::Choices(chosen)) => chosen.clone(),
                    _ => Vec::new(),
                };
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .children(options.iter().enumerate().map(|(index, option)| {
                        let selected = chosen.contains(option);
                        let memory = memory.clone();
                        let option = option.clone();
                        let chosen = chosen.clone();
                        option_row(
                            index,
                            option.clone(),
                            selected,
                            true,
                            Rc::new(move |_, cx| {
                                let mut next = chosen.clone();
                                if selected {
                                    next.retain(|choice| *choice != option);
                                } else {
                                    next.push(option.clone());
                                }
                                memory.update(cx, |memory| {
                                    memory.answers[memory.step] =
                                        (!next.is_empty()).then_some(Answer::Choices(next))
                                })
                            }),
                        )
                    }))
                    .into_any_element()
            }
            QuestionKind::Freeform => match freeform.as_ref() {
                Some(state) => Textarea::new(state).into_any_element(),
                None => div().into_any_element(),
            },
        };

        let freeform_text = freeform
            .as_ref()
            .map(|state| state.read(cx).text().trim().to_string());
        let can_continue = match &question.kind {
            QuestionKind::Freeform => freeform_text
                .as_deref()
                .is_some_and(|text| !text.is_empty()),
            _ => current_answer.is_some(),
        };
        let is_last = step + 1 == question_count;

        let back_memory = memory.clone();
        let skip = question.skippable.then(|| {
            let memory = memory.clone();
            let questions = questions.clone();
            let on_complete = on_complete.clone();
            Button::new("questionnaire-skip")
                .ghost()
                .label("Skip")
                .on_click(move |_, window, cx| {
                    advance(
                        &memory,
                        Answer::Skipped,
                        &questions,
                        on_complete.as_ref(),
                        window,
                        cx,
                    )
                })
        });
        let next = {
            let memory = memory.clone();
            let questions = questions.clone();
            let on_complete = on_complete.clone();
            let current_answer = current_answer.clone();
            let freeform_text = freeform_text.clone();
            Button::new("questionnaire-next")
                .label(if is_last { "Finish" } else { "Next" })
                .disabled(!can_continue)
                .on_click(move |_, window, cx| {
                    let answer = match (&freeform_text, &current_answer) {
                        (Some(text), _) => Answer::Text(text.clone().into()),
                        (None, Some(answer)) => answer.clone(),
                        (None, None) => return,
                    };
                    advance(
                        &memory,
                        answer,
                        &questions,
                        on_complete.as_ref(),
                        window,
                        cx,
                    )
                })
        };

        container
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.))
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(format!("Question {} of {}", step + 1, question_count)),
                    )
                    .child(Progress::new(step as f32 / question_count as f32 * 100.).h(px(4.))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(question.title.clone()),
                    )
                    .when_some(question.description.clone(), |header, description| {
                        header.child(
                            div()
                                .text_sm()
                                .text_color(colors.muted_foreground)
                                .child(description),
                        )
                    }),
            )
            .child(body)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .child(
                        Button::new("questionnaire-back")
                            .ghost()
                            .icon(IconName::ArrowLeft)
                            .label("Back")
                            .disabled(step == 0)
                            .on_click(move |_, _, cx| {
                                back_memory.update(cx, |memory| {
                                    memory.step = memory.step.saturating_sub(1)
                                })
                            }),
                    )
                    .child(div().flex_1())
                    .children(skip)
                    .child(next),
            )
            .apply_style_overrides(&self.style_overrides)
    }
}

#[cfg(test)]
mod tests {
    use super::Answer;

    #[test]
    fn answers_summarize() {
        assert_eq!(
            Answer::Choice("Engineer".into()).summary().as_ref(),
            "Engineer"
        );
        assert_eq!(
            Answer::Choices(vec!["Figma".into(), "Zed".into()])
                .summary()
                .as_ref(),
            "Figma, Zed"
        );
        assert_eq!(Answer::Skipped.summary().as_ref(), "Skipped");
    }
}
