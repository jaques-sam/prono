use std::collections::HashMap;
use std::ops::RangeInclusive;

use chrono::Datelike;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    /// Derived from the question text and answer kind, see `catalog`.
    pub id: Uuid,
    pub text: String,
    pub kind: AnswerKind,
    /// The options of a `Poll` question, empty for any other kind. Free
    /// "other" answers are always allowed on top of these.
    pub possible_answers: Vec<String>,
}

/// What kind of answer a question expects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnswerKind {
    Text,
    PredictionDate,
    Poll,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Answer {
    Text(String),
    PredictionDate {
        day: Option<u8>,
        month: u8,
        year: u16,
    },
    /// The text of the chosen option, or a free "other" text.
    Poll(String),
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
    pub answers: Vec<(Uuid, Answer)>,
}

/// One participant's answer to a question.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAnswer {
    pub user: String,
    pub answer: Answer,
}

/// Every participant's answers to a survey, keyed by question id.
pub type Results = HashMap<Uuid, Vec<UserAnswer>>;

impl Survey {
    /// A blank answer for each question, in question order.
    #[must_use]
    pub fn empty_answers(&self) -> Vec<Answer> {
        self.questions.iter().map(|q| q.kind.empty_answer()).collect()
    }
}

impl Survey {
    /// `user`'s answers to this survey in question order, if `results` hold a
    /// valid answer of theirs to every question: what to refill when they
    /// come back.
    #[must_use]
    pub fn answers_of(&self, results: &Results, user: &str) -> Option<Vec<Answer>> {
        self.questions
            .iter()
            .map(|question| {
                results
                    .get(&question.id)?
                    .iter()
                    .find(|found| found.user == user && found.answer.is_valid_for(question.kind))
                    .map(|found| found.answer.clone())
            })
            .collect()
    }
}

impl Question {
    /// The votes of `answers` (only `Answer::Poll` count): one entry per
    /// possible answer in their order, zero votes included, followed by every
    /// other voted text grouped by exact text, most votes first, then by text.
    #[must_use]
    pub fn poll_tally(&self, answers: &[UserAnswer]) -> Vec<(String, usize)> {
        let mut votes: HashMap<&str, usize> = HashMap::new();
        for user_answer in answers {
            if let Answer::Poll(text) = &user_answer.answer {
                *votes.entry(text).or_default() += 1;
            }
        }
        let mut tally: Vec<_> = self
            .possible_answers
            .iter()
            .map(|option| (option.clone(), votes.remove(option.as_str()).unwrap_or(0)))
            .collect();
        let mut others: Vec<_> = votes.into_iter().map(|(text, n)| (text.to_string(), n)).collect();
        others.sort_by(|(a, count_a), (b, count_b)| count_b.cmp(count_a).then_with(|| a.cmp(b)));
        tally.extend(others);
        tally
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
            AnswerKind::Poll => Answer::Poll(String::new()),
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
            Answer::Poll(_) => AnswerKind::Poll,
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
            Answer::Text(text) | Answer::Poll(text) => {
                !text.trim().is_empty() && text.chars().count() <= MAX_TEXT_ANSWER_LEN
            }
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
                .map(|(question, answer)| (question.id, answer.clone()))
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

    const Q1: Uuid = Uuid::from_u128(1);
    const Q2: Uuid = Uuid::from_u128(2);
    const Q9: Uuid = Uuid::from_u128(9);

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
                    id: Q1,
                    text: "When?".into(),
                    kind: AnswerKind::PredictionDate,
                    possible_answers: Vec::new(),
                },
                Question {
                    id: Q2,
                    text: "Why?".into(),
                    kind: AnswerKind::Text,
                    possible_answers: Vec::new(),
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
    fn poll_answers_are_valid_like_text() {
        assert_eq!(AnswerKind::Poll.empty_answer(), Answer::Poll(String::new()));
        assert!(!AnswerKind::Poll.empty_answer().is_valid_for(AnswerKind::Poll));
        assert!(Answer::Poll("C++".into()).is_valid_for(AnswerKind::Poll));
        assert!(Answer::Poll("something else".into()).is_valid_for(AnswerKind::Poll));
        assert!(!Answer::Poll("  ".into()).is_valid_for(AnswerKind::Poll));
        assert!(!Answer::Poll("x".repeat(MAX_TEXT_ANSWER_LEN + 1)).is_valid_for(AnswerKind::Poll));
        assert!(!Answer::Poll("C".into()).is_valid_for(AnswerKind::Text));
        assert!(!Answer::Text("C".into()).is_valid_for(AnswerKind::Poll));
    }

    #[test]
    fn poll_answer_wire_format() {
        let json = serde_json::to_string(&Answer::Poll("C++".into())).unwrap();
        assert_eq!(json, r#"{"Poll":"C++"}"#);
        assert_eq!(
            serde_json::from_str::<Answer>(&json).unwrap(),
            Answer::Poll("C++".into())
        );
    }

    fn poll_question() -> Question {
        Question {
            id: Q1,
            text: "Background?".into(),
            kind: AnswerKind::Poll,
            possible_answers: vec!["C".into(), "C++".into(), "Rust".into()],
        }
    }

    fn voted(user: &str, answer: Answer) -> UserAnswer {
        UserAnswer {
            user: user.into(),
            answer,
        }
    }

    #[test]
    fn poll_tally_lists_options_in_order_then_others_by_votes() {
        let poll = |text: &str| Answer::Poll(text.into());
        let answers = [
            voted("a", poll("Rust")),
            voted("b", poll("Rust")),
            voted("c", poll("C")),
            voted("d", poll("Go")),
            voted("e", poll("Zig")),
            voted("f", poll("Zig")),
            voted("g", poll("Ada")),
            // Not a poll answer: ignored.
            voted("h", Answer::Text("C++".into())),
            // Matching is exact, so this is an "other" answer.
            voted("i", poll("rust")),
        ];
        assert_eq!(
            poll_question().poll_tally(&answers),
            [
                ("C".to_string(), 1),
                ("C++".to_string(), 0),
                ("Rust".to_string(), 2),
                ("Zig".to_string(), 2),
                ("Ada".to_string(), 1),
                ("Go".to_string(), 1),
                ("rust".to_string(), 1),
            ]
        );
    }

    #[test]
    fn poll_tally_without_votes_has_every_option_at_zero() {
        assert_eq!(
            poll_question().poll_tally(&[]),
            [("C".to_string(), 0), ("C++".to_string(), 0), ("Rust".to_string(), 0)]
        );
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
        assert_eq!(submission.answers[0], (Q1, date(5, 2030)));
        assert_eq!(submission.validate(&survey), Ok(()));
    }

    #[test]
    fn submission_must_answer_every_question_once() {
        let survey = survey();
        let missing = Submission {
            user: "Sam".into(),
            answers: vec![(Q1, date(5, 2030))],
        };
        assert_eq!(missing.validate(&survey), Err(Invalid::Incomplete));

        let duplicate = Submission {
            user: "Sam".into(),
            answers: vec![(Q1, date(5, 2030)), (Q1, date(6, 2030))],
        };
        assert_eq!(duplicate.validate(&survey), Err(Invalid::Incomplete));

        let unknown = Submission {
            user: "Sam".into(),
            answers: vec![(Q1, date(5, 2030)), (Q9, Answer::Text("x".into()))],
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

    fn results_of(entries: &[(Uuid, &str, Answer)]) -> Results {
        let mut results = Results::new();
        for (question, user, answer) in entries {
            results.entry(*question).or_default().push(UserAnswer {
                user: (*user).into(),
                answer: answer.clone(),
            });
        }
        results
    }

    #[test]
    fn answers_of_picks_the_users_answers_in_question_order() {
        let results = results_of(&[
            (Q2, "Bob", Answer::Text("bob".into())),
            (Q2, "Sam", Answer::Text("sam".into())),
            (Q1, "Bob", date(1, 2031)),
            (Q1, "Sam", date(5, 2030)),
        ]);
        assert_eq!(
            survey().answers_of(&results, "Sam"),
            Some(vec![date(5, 2030), Answer::Text("sam".into())])
        );
    }

    #[test]
    fn answers_of_needs_a_valid_answer_to_every_question() {
        let partial = results_of(&[(Q1, "Sam", date(5, 2030))]);
        assert_eq!(survey().answers_of(&partial, "Sam"), None);

        let wrong_kind = results_of(&[(Q1, "Sam", date(5, 2030)), (Q2, "Sam", date(5, 2030))]);
        assert_eq!(survey().answers_of(&wrong_kind, "Sam"), None);

        let other_user = results_of(&[(Q1, "Bob", date(5, 2030)), (Q2, "Bob", Answer::Text("x".into()))]);
        assert_eq!(survey().answers_of(&other_user, "Sam"), None);
        assert_eq!(survey().answers_of(&Results::new(), "Sam"), None);
    }

    #[test]
    fn question_ids_keep_their_wire_format() {
        // Typed ids are still hyphenated strings on the wire, as in the deployed
        // web app and backend, also as keys of the results.
        let id = "7873dd07-86a3-593b-ab8f-80bce8b7e84e";
        let uuid: Uuid = id.parse().unwrap();

        let submission = Submission {
            user: "Sam".into(),
            answers: vec![(uuid, Answer::Text("x".into()))],
        };
        let json = serde_json::to_string(&submission).unwrap();
        assert_eq!(json, format!(r#"{{"user":"Sam","answers":[["{id}",{{"Text":"x"}}]]}}"#));
        assert_eq!(serde_json::from_str::<Submission>(&json).unwrap(), submission);

        let results = format!(r#"{{"{id}":[{{"user":"Sam","answer":{{"Text":"x"}}}}]}}"#);
        let parsed: Results = serde_json::from_str(&results).unwrap();
        assert_eq!(parsed[&uuid][0].user, "Sam");
        assert_eq!(serde_json::to_string(&parsed).unwrap(), results);
    }

    #[test]
    fn malformed_question_ids_do_not_parse() {
        let body = r#"{"user":"Sam","answers":[["not-a-uuid",{"Text":"x"}]]}"#;
        assert!(serde_json::from_str::<Submission>(body).is_err());
    }
}
