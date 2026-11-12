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

/// A questionnaire, uploaded to the backend as a JSON definition (see
/// [`Survey::from_json`]) and sent from there to the app.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Survey {
    pub id: u64,
    /// The short name offered when choosing a survey.
    pub title: String,
    /// What the survey is about, shown once it is opened.
    pub description: String,
    pub link: Option<String>,
    /// Whether every question has a correct answer, so the participants get
    /// a [`Score`]. The correct answers themselves never leave the backend.
    pub scored: bool,
    pub questions: Vec<Question>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    /// Derived from the question text and answer kind, see `definition`.
    pub id: Uuid,
    pub text: String,
    pub kind: AnswerKind,
    /// The options of a `Poll` question, empty for any other kind. Free
    /// "other" answers are always allowed on top of these.
    pub possible_answers: Vec<String>,
    /// What the free answer of a `Poll` question is called, when "Other"
    /// does not fit: "Yes, namely:" next to the option "No".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_label: Option<String>,
    /// What a scored survey counts as right. A secret of the backend: it is
    /// never serialized, so it cannot reach the app.
    #[serde(default, skip_serializing)]
    pub correct_answer: Option<Answer>,
}

/// What kind of answer a question expects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    #[error("Please fill in your first and last name (at most {MAX_USER_NAME_LEN} characters)")]
    UserName,
    #[error("Unknown survey {0}")]
    UnknownSurvey(u64),
    #[error("Please answer every question of the survey exactly once")]
    Incomplete,
    #[error("Invalid answer to \"{0}\"")]
    Answer(String),
    #[error("Survey {0} has no scores")]
    NotScored(u64),
}

/// A participant's answers to one survey: the body of a submit request.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Submission {
    /// The participant's [`full_name`].
    pub user: String,
    /// The survey's pass phrase: without the right one the backend refuses
    /// the submission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pass_phrase: Option<String>,
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

/// What a participant got right in a scored survey.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Score {
    pub user: String,
    /// The number of correct answers.
    pub points: usize,
}

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

impl Survey {
    /// What each participant got right, best first, then by name. Empty for
    /// a survey that is not [`scored`](Survey::scored). Only a survey that
    /// knows its correct answers (the backend's) can score.
    #[must_use]
    pub fn scores(&self, results: &Results) -> Vec<Score> {
        let mut points: HashMap<&str, usize> = HashMap::new();
        for question in &self.questions {
            let Some(correct) = &question.correct_answer else {
                continue;
            };
            for user_answer in results.get(&question.id).map_or(&[][..], Vec::as_slice) {
                let right = usize::from(user_answer.answer.is_correct(correct));
                *points.entry(&user_answer.user).or_default() += right;
            }
        }
        let mut scores: Vec<_> = points
            .into_iter()
            .map(|(user, points)| Score {
                user: user.to_string(),
                points,
            })
            .collect();
        scores.sort_by(|a, b| b.points.cmp(&a.points).then_with(|| a.user.cmp(&b.user)));
        scores
    }
}

impl Question {
    /// What the free answer of a poll is called: [`Self::other_label`], or
    /// "Other".
    #[must_use]
    pub fn other_label(&self) -> &str {
        self.other_label.as_deref().unwrap_or("Other")
    }

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

    /// Whether this answer is as good as `correct`: texts and poll choices
    /// match ignoring case and surrounding spaces, dates by month and year.
    #[must_use]
    pub fn is_correct(&self, correct: &Answer) -> bool {
        match (self, correct) {
            (Answer::Text(a), Answer::Text(b)) | (Answer::Poll(a), Answer::Poll(b)) => {
                a.trim().to_lowercase() == b.trim().to_lowercase()
            }
            (
                Answer::PredictionDate { month, year, .. },
                Answer::PredictionDate {
                    month: correct_month,
                    year: correct_year,
                    ..
                },
            ) => month == correct_month && year == correct_year,
            _ => false,
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

/// The answer as a person reads it: the text itself, or a date as `DD/MM/YYYY`
/// (`MM/YYYY` without a day).
impl std::fmt::Display for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Answer::Text(text) | Answer::Poll(text) => f.write_str(text),
            Answer::PredictionDate {
                day: Some(day),
                month,
                year,
            } => write!(f, "{day:02}/{month:02}/{year:04}"),
            Answer::PredictionDate { day: None, month, year } => write!(f, "{month:02}/{year:04}"),
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

/// A participant's name as it is stored: the trimmed first and last name,
/// joined by a space.
#[must_use]
pub fn full_name(first: &str, last: &str) -> String {
    format!("{} {}", first.trim(), last.trim())
}

/// Checks a participant's first and last name and returns their [`full_name`].
///
/// # Errors
///
/// [`Invalid::UserName`] if either is blank, or the full name is too long.
pub fn check_name_parts(first: &str, last: &str) -> Result<String, Invalid> {
    if first.trim().is_empty() || last.trim().is_empty() {
        return Err(Invalid::UserName);
    }
    let name = full_name(first, last);
    check_user_name(&name)?;
    Ok(name)
}

/// The pass phrase as the backend compares it: without the surrounding
/// blanks, and `None` for a blank one.
#[must_use]
pub fn clean_pass_phrase(pass_phrase: &str) -> Option<String> {
    let pass_phrase = pass_phrase.trim();
    (!pass_phrase.is_empty()).then(|| pass_phrase.to_string())
}

impl Submission {
    /// Pairs `answers` (in question order) with `survey`'s questions.
    #[must_use]
    pub fn new(user: &str, survey: &Survey, answers: &[Answer]) -> Self {
        Self {
            user: user.trim().to_string(),
            pass_phrase: None,
            answers: survey
                .questions
                .iter()
                .zip(answers)
                .map(|(question, answer)| (question.id, answer.clone()))
                .collect(),
        }
    }

    /// The submission with the pass phrase typed by the participant; a blank
    /// one counts as none.
    #[must_use]
    pub fn with_pass_phrase(mut self, pass_phrase: &str) -> Self {
        self.pass_phrase = clean_pass_phrase(pass_phrase);
        self
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
            title: "Test".into(),
            description: "Test".into(),
            link: None,
            scored: false,
            questions: vec![
                Question {
                    id: Q1,
                    text: "When?".into(),
                    kind: AnswerKind::PredictionDate,
                    possible_answers: Vec::new(),
                    other_label: None,
                    correct_answer: None,
                },
                Question {
                    id: Q2,
                    text: "Why?".into(),
                    kind: AnswerKind::Text,
                    possible_answers: Vec::new(),
                    other_label: None,
                    correct_answer: None,
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
            other_label: None,
            correct_answer: None,
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
            pass_phrase: None,
            answers: vec![(Q1, date(5, 2030))],
        };
        assert_eq!(missing.validate(&survey), Err(Invalid::Incomplete));

        let duplicate = Submission {
            user: "Sam".into(),
            pass_phrase: None,
            answers: vec![(Q1, date(5, 2030)), (Q1, date(6, 2030))],
        };
        assert_eq!(duplicate.validate(&survey), Err(Invalid::Incomplete));

        let unknown = Submission {
            user: "Sam".into(),
            pass_phrase: None,
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
            pass_phrase: None,
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

    #[test]
    fn name_parts_make_one_full_name() {
        assert_eq!(full_name(" Ada ", "Lovelace "), "Ada Lovelace");
        assert_eq!(check_name_parts("Ada", "Lovelace"), Ok("Ada Lovelace".into()));
        assert_eq!(check_name_parts("Ada", "  "), Err(Invalid::UserName));
        assert_eq!(check_name_parts("", "Lovelace"), Err(Invalid::UserName));
        let long = "x".repeat(MAX_USER_NAME_LEN);
        assert_eq!(check_name_parts(&long, "y"), Err(Invalid::UserName));
    }

    #[test]
    fn pass_phrase_is_trimmed_and_optional_on_the_wire() {
        let survey = survey();
        let plain = Submission::new("Sam", &survey, &survey.empty_answers());
        assert_eq!(plain.pass_phrase, None);
        // Submissions of the deployed app have no such key.
        assert!(!serde_json::to_string(&plain).unwrap().contains("pass_phrase"));
        let old = r#"{"user":"Sam","answers":[]}"#;
        assert_eq!(serde_json::from_str::<Submission>(old).unwrap().pass_phrase, None);

        let with = plain.clone().with_pass_phrase("  open sesame ");
        assert_eq!(with.pass_phrase.as_deref(), Some("open sesame"));
        assert_eq!(plain.with_pass_phrase("   ").pass_phrase, None);
        assert_eq!(clean_pass_phrase(" open sesame\n").as_deref(), Some("open sesame"));
        assert_eq!(clean_pass_phrase(""), None);
    }

    fn quiz() -> Survey {
        let mut survey = survey();
        survey.scored = true;
        survey.questions[0].correct_answer = Some(date(7, 2028));
        survey.questions[1].correct_answer = Some(Answer::Text("Because".into()));
        survey
    }

    #[test]
    fn answers_display_as_a_person_reads_them() {
        assert_eq!(Answer::Text("Because".into()).to_string(), "Because");
        assert_eq!(Answer::Poll("C++".into()).to_string(), "C++");
        assert_eq!(date(7, 2028).to_string(), "07/2028");
        let with_day = Answer::PredictionDate {
            day: Some(3),
            month: 7,
            year: 2028,
        };
        assert_eq!(with_day.to_string(), "03/07/2028");
    }

    #[test]
    fn answers_are_correct_ignoring_case_spaces_and_the_day() {
        let text = |s: &str| Answer::Text(s.into());
        assert!(text(" because ").is_correct(&text("Because")));
        assert!(!text("because not").is_correct(&text("Because")));
        let poll = |s: &str| Answer::Poll(s.into());
        assert!(poll("c++").is_correct(&poll("C++")));
        assert!(!poll("C").is_correct(&poll("C++")));

        let with_day = Answer::PredictionDate {
            day: Some(3),
            month: 7,
            year: 2028,
        };
        assert!(with_day.is_correct(&date(7, 2028)));
        assert!(date(7, 2028).is_correct(&with_day));
        assert!(!date(8, 2028).is_correct(&date(7, 2028)));
        assert!(!date(7, 2029).is_correct(&date(7, 2028)));

        // Another kind is never correct, even with the same text.
        assert!(!text("C").is_correct(&poll("C")));
        assert!(!date(7, 2028).is_correct(&text("7/2028")));
    }

    #[test]
    fn scores_count_the_correct_answers_best_first_then_by_name() {
        let right = |user, q1, q2| [(Q1, user, q1), (Q2, user, q2)];
        let results = results_of(
            &[
                right("Zed", date(7, 2028), Answer::Text("because".into())),
                right("Bob", date(1, 2030), Answer::Text("because".into())),
                right("Ann", date(1, 2030), Answer::Text("nope".into())),
                right("Cat", date(7, 2028), Answer::Text("BECAUSE".into())),
            ]
            .concat(),
        );
        let points = |user: &str, points| Score {
            user: user.into(),
            points,
        };
        assert_eq!(
            quiz().scores(&results),
            [points("Cat", 2), points("Zed", 2), points("Bob", 1), points("Ann", 0)]
        );
        assert_eq!(quiz().scores(&Results::new()), Vec::<Score>::new());
    }

    #[test]
    fn a_survey_without_correct_answers_has_no_scores() {
        let results = results_of(&[(Q1, "Sam", date(5, 2030)), (Q2, "Sam", Answer::Text("x".into()))]);
        assert_eq!(survey().scores(&results), Vec::<Score>::new());
    }

    #[test]
    fn correct_answers_never_reach_the_wire() {
        let json = serde_json::to_string(&quiz()).unwrap();
        assert!(!json.contains("correct_answer"));
        assert!(!json.contains("Because"));
        let sent: Survey = serde_json::from_str(&json).unwrap();
        assert!(sent.scored, "the app still learns that it is scored");
        assert!(sent.questions.iter().all(|q| q.correct_answer.is_none()));
    }
}
