use std::collections::HashMap;
use std::ops::RangeInclusive;

use chrono::Datelike;
use serde::{Deserialize, Serialize};

/// Years a predicted date may fall in.
pub const PREDICTION_YEARS: RangeInclusive<u16> = 2024..=2100;
/// Longest accepted user name, in characters.
pub const MAX_USER_NAME_LEN: usize = 64;
/// Longest accepted text answer, in characters.
pub const MAX_TEXT_ANSWER_LEN: usize = 1000;

/// A questionnaire, loaded from the survey catalog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Survey {
    pub id: u64,
    pub description: String,
    pub link: Option<String>,
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub kind: AnswerKind,
}

/// What kind of answer a question expects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerKind {
    Text,
    PredictionDate,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Answer {
    Text(String),
    PredictionDate { day: Option<u8>, month: u8, year: u16 },
}

/// Why a submission (or part of it) is rejected. The message is meant for
/// the participant.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum Invalid {
    #[error("Please fill in your name (at most {MAX_USER_NAME_LEN} characters)")]
    UserName,
    #[error("Unknown survey {0}")]
    UnknownSurvey(u64),
    #[error("Please answer every question of the survey exactly once")]
    Incomplete,
    #[error("Invalid answer to \"{0}\"")]
    Answer(String),
}

/// A participant's answers to one survey: the body of a submit request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    pub user: String,
    /// `(question id, answer)` pairs.
    pub answers: Vec<(String, Answer)>,
}

/// One participant's answer to a question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAnswer {
    pub user: String,
    pub answer: Answer,
}

/// Every participant's answers to a survey, keyed by question id.
pub type Results = HashMap<String, Vec<UserAnswer>>;

impl Survey {
    /// A blank answer for each question, in question order.
    #[must_use]
    pub fn empty_answers(&self) -> Vec<Answer> {
        self.questions.iter().map(|q| q.kind.empty_answer()).collect()
    }
}

impl AnswerKind {
    /// The answer a question starts with: empty text, or the current month.
    ///
    /// # Panics
    ///
    /// Never in practice: the current month and year always fit.
    #[must_use]
    pub fn empty_answer(self) -> Answer {
        match self {
            AnswerKind::Text => Answer::Text(String::new()),
            AnswerKind::PredictionDate => {
                let today = chrono::Utc::now();
                Answer::PredictionDate {
                    day: None,
                    month: u8::try_from(today.month()).expect("month fits in u8"),
                    year: u16::try_from(today.year()).expect("year fits in u16"),
                }
            }
        }
    }
}

impl Answer {
    #[must_use]
    pub fn kind(&self) -> AnswerKind {
        match self {
            Answer::Text(_) => AnswerKind::Text,
            Answer::PredictionDate { .. } => AnswerKind::PredictionDate,
        }
    }

    /// Whether this is a well-formed answer to a question of kind `kind`.
    #[must_use]
    pub fn is_valid_for(&self, kind: AnswerKind) -> bool {
        if self.kind() != kind {
            return false;
        }
        match self {
            Answer::Text(text) => !text.trim().is_empty() && text.chars().count() <= MAX_TEXT_ANSWER_LEN,
            Answer::PredictionDate { day, month, year } => {
                (1..=12).contains(month)
                    && PREDICTION_YEARS.contains(year)
                    && day.is_none_or(|day| (1..=31).contains(&day))
            }
        }
    }
}

/// Checks a participant's name.
///
/// # Errors
///
/// [`Invalid::UserName`] if it is empty, too long, or has surrounding spaces.
pub fn check_user_name(name: &str) -> Result<(), Invalid> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed != name || name.chars().count() > MAX_USER_NAME_LEN {
        return Err(Invalid::UserName);
    }
    Ok(())
}

impl Submission {
    /// Pairs `answers` (in question order) with `survey`'s questions.
    #[must_use]
    pub fn new(user: &str, survey: &Survey, answers: &[Answer]) -> Self {
        Self {
            user: user.trim().to_string(),
            answers: survey
                .questions
                .iter()
                .zip(answers)
                .map(|(question, answer)| (question.id.clone(), answer.clone()))
                .collect(),
        }
    }

    /// Checks the submission against `survey`: a valid name and exactly one
    /// valid answer per question.
    ///
    /// # Errors
    ///
    /// The first problem found, as an [`Invalid`].
    pub fn validate(&self, survey: &Survey) -> Result<(), Invalid> {
        check_user_name(&self.user)?;
        if self.answers.len() != survey.questions.len() {
            return Err(Invalid::Incomplete);
        }
        for question in &survey.questions {
            let answer = self
                .answers
                .iter()
                .find_map(|(id, answer)| (*id == question.id).then_some(answer))
                .ok_or(Invalid::Incomplete)?;
            if !answer.is_valid_for(question.kind) {
                return Err(Invalid::Answer(question.text.clone()));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(month: u8, year: u16) -> Answer {
        Answer::PredictionDate { day: None, month, year }
    }

    fn survey() -> Survey {
        Survey {
            id: 7,
            description: "Test".into(),
            link: None,
            questions: vec![
                Question {
                    id: "q1".into(),
                    text: "When?".into(),
                    kind: AnswerKind::PredictionDate,
                },
                Question {
                    id: "q2".into(),
                    text: "Why?".into(),
                    kind: AnswerKind::Text,
                },
            ],
        }
    }

    #[test]
    fn empty_answers_match_question_kinds() {
        let answers = survey().empty_answers();
        assert!(matches!(answers[0], Answer::PredictionDate { day: None, .. }));
        assert_eq!(answers[1], Answer::Text(String::new()));
    }

    #[test]
    fn empty_prediction_is_valid() {
        assert!(
            AnswerKind::PredictionDate
                .empty_answer()
                .is_valid_for(AnswerKind::PredictionDate)
        );
    }

    #[test]
    fn answer_validity() {
        assert!(date(12, 2030).is_valid_for(AnswerKind::PredictionDate));
        assert!(!date(0, 2030).is_valid_for(AnswerKind::PredictionDate));
        assert!(!date(13, 2030).is_valid_for(AnswerKind::PredictionDate));
        assert!(!date(5, 1999).is_valid_for(AnswerKind::PredictionDate));
        assert!(!date(5, 2030).is_valid_for(AnswerKind::Text));
        let bad_day = Answer::PredictionDate {
            day: Some(32),
            month: 1,
            year: 2030,
        };
        assert!(!bad_day.is_valid_for(AnswerKind::PredictionDate));

        assert!(Answer::Text("because".into()).is_valid_for(AnswerKind::Text));
        assert!(!Answer::Text("  ".into()).is_valid_for(AnswerKind::Text));
        assert!(!Answer::Text("x".repeat(MAX_TEXT_ANSWER_LEN + 1)).is_valid_for(AnswerKind::Text));
    }

    #[test]
    fn user_name_rules() {
        assert_eq!(check_user_name("Sam"), Ok(()));
        assert_eq!(check_user_name(""), Err(Invalid::UserName));
        assert_eq!(check_user_name("  "), Err(Invalid::UserName));
        assert_eq!(check_user_name(" Sam"), Err(Invalid::UserName));
        assert_eq!(
            check_user_name(&"x".repeat(MAX_USER_NAME_LEN + 1)),
            Err(Invalid::UserName)
        );
    }

    #[test]
    fn new_submission_trims_name_and_pairs_answers() {
        let survey = survey();
        let submission = Submission::new(" Sam ", &survey, &[date(5, 2030), Answer::Text("fun".into())]);
        assert_eq!(submission.user, "Sam");
        assert_eq!(submission.answers[0], ("q1".into(), date(5, 2030)));
        assert_eq!(submission.validate(&survey), Ok(()));
    }

    #[test]
    fn submission_must_answer_every_question_once() {
        let survey = survey();
        let missing = Submission {
            user: "Sam".into(),
            answers: vec![("q1".into(), date(5, 2030))],
        };
        assert_eq!(missing.validate(&survey), Err(Invalid::Incomplete));

        let duplicate = Submission {
            user: "Sam".into(),
            answers: vec![("q1".into(), date(5, 2030)), ("q1".into(), date(6, 2030))],
        };
        assert_eq!(duplicate.validate(&survey), Err(Invalid::Incomplete));

        let unknown = Submission {
            user: "Sam".into(),
            answers: vec![("q1".into(), date(5, 2030)), ("q9".into(), Answer::Text("x".into()))],
        };
        assert_eq!(unknown.validate(&survey), Err(Invalid::Incomplete));
    }

    #[test]
    fn submission_rejects_invalid_answer_and_name() {
        let survey = survey();
        let blank_text = Submission::new("Sam", &survey, &survey.empty_answers());
        assert_eq!(blank_text.validate(&survey), Err(Invalid::Answer("Why?".into())));

        let no_name = Submission::new("", &survey, &[date(5, 2030), Answer::Text("x".into())]);
        assert_eq!(no_name.validate(&survey), Err(Invalid::UserName));
    }

    #[test]
    fn answer_wire_format_is_unchanged() {
        // The deployed web app and backend exchange answers in this shape.
        let json = serde_json::to_string(&date(3, 2026)).unwrap();
        assert_eq!(json, r#"{"PredictionDate":{"day":null,"month":3,"year":2026}}"#);
    }
}
