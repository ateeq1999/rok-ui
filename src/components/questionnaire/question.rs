//! Questions and answers: what a [`Questionnaire`](super::Questionnaire) asks and collects.

#[cfg(doc)]
use super::Questionnaire;
use gpui::SharedString;

#[derive(Clone)]
pub(super) enum QuestionKind {
    Single(Vec<SharedString>),
    Multiple(Vec<SharedString>),
    Freeform,
}

/// One step of a [`Questionnaire`].
#[derive(Clone)]
pub struct Question {
    pub(super) id: SharedString,
    pub(super) title: SharedString,
    pub(super) description: Option<SharedString>,
    pub(super) kind: QuestionKind,
    pub(super) skippable: bool,
    pub(super) placeholder: SharedString,
}

impl Question {
    pub(super) fn new(
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

    /// Help text below the question.
    #[must_use]
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Offer a Skip button.
    #[must_use]
    pub fn skippable(mut self) -> Self {
        self.skippable = true;
        self
    }

    /// Placeholder of a freeform answer.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }
}

/// The answer to one question.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// One option of a single-choice question.
    Choice(SharedString),
    /// The options picked in a multiple-choice question.
    Choices(Vec<SharedString>),
    /// Free text.
    Text(SharedString),
    /// The question was skipped.
    Skipped,
}

impl Answer {
    /// A one-line summary for review screens.
    #[must_use]
    pub fn summary(&self) -> SharedString {
        match self {
            Answer::Choice(choice) => choice.clone(),
            Answer::Choices(choices) => choices
                .iter()
                .map(std::convert::AsRef::as_ref)
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
    /// The id of the question.
    pub question_id: SharedString,
    /// What the user answered.
    pub answer: Answer,
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
