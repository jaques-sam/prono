//! The survey definition: the JSON an author uploads to the backend, and how
//! it becomes a [`Survey`].
//!
//! ```json
//! {
//!     "survey_id": 1,
//!     "description": "What the survey is about",
//!     "link": "https://example.com",
//!     "questions": [
//!         { "question": "When?", "answer_type": "prediction_date",
//!           "correct_answer": { "month": 7, "year": 2028 } },
//!         { "question": "Why?", "answer_type": "text", "correct_answer": "because" },
//!         { "question": "Which?", "answer_type": "poll",
//!           "possible_answers": ["A", "B"], "correct_answer": "A" },
//!         { "question": "Any allergies?", "answer_type": "poll",
//!           "possible_answers": ["No"], "other_label": "Yes, namely:" }
//!     ]
//! }
//! ```
//!
//! `link` is optional. There is no access key: every survey is opened by its
//! pass phrase, which is set with `prono-cli surveys pass-phrase` and lives in
//! the backend's database, never in the definition. `possible_answers` is for `poll` questions only, and so is `other_label`:
//! the name of the free answer, "Other" when left out. A `correct_answer`
//! makes the survey scored, and then every question needs one.

use std::collections::HashSet;

use serde::Deserialize;
use uuid::Uuid;

use crate::{Answer, AnswerKind, Question, Survey};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SurveyFile {
    survey_id: u64,
    description: String,
    #[serde(default)]
    link: Option<String>,
    questions: Vec<QuestionFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestionFile {
    question: String,
    answer_type: AnswerKind,
    /// The options of a `poll` question, and only of those.
    #[serde(default)]
    possible_answers: Vec<String>,
    /// The name of the free answer of a `poll` question, and only of those.
    #[serde(default)]
    other_label: Option<String>,
    #[serde(default)]
    correct_answer: Option<CorrectAnswerFile>,
}

/// A correct answer as written in the file: text for a `text` or `poll`
/// question, a month and a year for a `prediction_date`.
#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum CorrectAnswerFile {
    Text(String),
    Date { month: u8, year: u16 },
}

impl QuestionFile {
    fn check<E: serde::de::Error>(&self) -> Result<(), E> {
        let options = &self.possible_answers;
        let is_poll = self.answer_type == AnswerKind::Poll;
        if is_poll && options.is_empty() {
            return Err(E::custom(format!(
                "poll question \"{}\" needs possible_answers",
                self.question
            )));
        }
        if !is_poll && !options.is_empty() {
            return Err(E::custom(format!(
                "question \"{}\" is not a poll, it cannot have possible_answers",
                self.question
            )));
        }
        let mut seen = HashSet::new();
        for option in options {
            if option.trim() != option || option.is_empty() || !seen.insert(option) {
                return Err(E::custom(format!(
                    "possible answer \"{option}\" of \"{}\" must be non-blank, trimmed and unique",
                    self.question
                )));
            }
        }
        if let Some(label) = &self.other_label {
            if !is_poll {
                return Err(E::custom(format!(
                    "question \"{}\" is not a poll, it cannot have an other_label",
                    self.question
                )));
            }
            if label.trim() != label || label.is_empty() {
                return Err(E::custom(format!(
                    "the other_label \"{label}\" of \"{}\" must be non-blank and trimmed",
                    self.question
                )));
            }
        }
        Ok(())
    }

    /// The correct answer as an [`Answer`] of the question's kind.
    fn correct_answer<E: serde::de::Error>(&self) -> Result<Option<Answer>, E> {
        let invalid = || {
            E::custom(format!(
                "the correct answer of \"{}\" does not fit a {:?} question",
                self.question, self.answer_type
            ))
        };
        let answer = match (&self.correct_answer, self.answer_type) {
            (None, _) => return Ok(None),
            (Some(CorrectAnswerFile::Text(text)), AnswerKind::Text) => Answer::Text(text.clone()),
            (Some(CorrectAnswerFile::Text(text)), AnswerKind::Poll) => {
                if !self.possible_answers.contains(text) {
                    return Err(E::custom(format!(
                        "the correct answer of \"{}\" must be one of its possible_answers",
                        self.question
                    )));
                }
                Answer::Poll(text.clone())
            }
            (Some(CorrectAnswerFile::Date { month, year }), AnswerKind::PredictionDate) => Answer::PredictionDate {
                day: None,
                month: *month,
                year: *year,
            },
            _ => return Err(invalid()),
        };
        if answer.is_valid_for(self.answer_type) {
            Ok(Some(answer))
        } else {
            Err(invalid())
        }
    }
}

impl Survey {
    /// Reads a survey definition (see the [module](self) docs).
    ///
    /// # Errors
    ///
    /// If `json` is no valid definition: malformed JSON, an unknown or
    /// missing key (also an `access` key, which no longer exists), bad `possible_answers`, or
    /// `correct_answer`s that do not fit their questions or are not given to
    /// every question or none.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        use serde::de::Error;

        let file: SurveyFile = serde_json::from_str(json)?;
        file.questions.iter().try_for_each(QuestionFile::check)?;
        let questions = file
            .questions
            .iter()
            .map(|q| {
                Ok(Question {
                    id: question_id(&q.question, q.answer_type),
                    text: q.question.clone(),
                    kind: q.answer_type,
                    possible_answers: q.possible_answers.clone(),
                    other_label: q.other_label.clone(),
                    correct_answer: q.correct_answer()?,
                })
            })
            .collect::<serde_json::Result<Vec<_>>>()?;

        let with_answer = questions.iter().filter(|q| q.correct_answer.is_some()).count();
        if with_answer != 0 && with_answer != questions.len() {
            return Err(serde_json::Error::custom(
                "a scored survey needs a correct_answer for every question",
            ));
        }
        Ok(Survey {
            id: file.survey_id,
            description: file.description,
            link: file.link,
            scored: with_answer != 0,
            questions,
        })
    }
}

/// Stored answers are keyed by this id, so it must never change for an
/// existing question: it hashes the question text and the answer kind.
fn question_id(text: &str, kind: AnswerKind) -> Uuid {
    let kind = match kind {
        AnswerKind::Text => "Text",
        AnswerKind::PredictionDate => "PredictionDate",
        AnswerKind::Poll => "Poll",
    };
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("{text}{kind}").as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples;

    #[test]
    fn the_shipped_surveys_parse() {
        assert_eq!(samples::starship().questions.len(), 4);
        assert_eq!(samples::course().questions.len(), 4);
        assert_ne!(samples::starship().id, samples::course().id);
    }

    #[test]
    fn existing_question_ids_are_stable() {
        // Answers already stored in the database reference these ids.
        let ids: Vec<_> = samples::starship().questions.iter().map(|q| q.id.to_string()).collect();
        assert_eq!(
            ids,
            [
                "7873dd07-86a3-593b-ab8f-80bce8b7e84e",
                "68bcd727-1c0b-5c4b-8b56-515657894205",
                "6ae332ad-b583-5748-97ff-65b13d86b42a",
                "84938a2e-9e40-562c-a68d-d33604ffac14",
            ]
        );
    }

    #[test]
    fn question_id_depends_on_text_and_kind() {
        let base = question_id("When?", AnswerKind::Text);
        assert_eq!(base, question_id("When?", AnswerKind::Text));
        assert_ne!(base, question_id("When", AnswerKind::Text));
        assert_ne!(base, question_id("When?", AnswerKind::PredictionDate));
        assert_ne!(base, question_id("When?", AnswerKind::Poll));
    }

    fn survey_json(question: &str) -> String {
        format!(r#"{{"survey_id": 1, "description": "", "questions": [{question}]}}"#)
    }

    #[test]
    fn poll_questions_carry_their_possible_answers() {
        let json = survey_json(r#"{"question": "Q?", "answer_type": "poll", "possible_answers": ["A", "B"]}"#);
        let question = &Survey::from_json(&json).unwrap().questions[0];
        assert_eq!(question.kind, AnswerKind::Poll);
        assert_eq!(question.possible_answers, ["A", "B"]);
        assert_eq!(question.id, question_id("Q?", AnswerKind::Poll));

        let text = survey_json(r#"{"question": "Q?", "answer_type": "text"}"#);
        assert_eq!(
            Survey::from_json(&text).unwrap().questions[0].possible_answers,
            Vec::<String>::new()
        );
    }

    #[test]
    fn invalid_possible_answers_are_rejected() {
        for question in [
            r#"{"question": "Q?", "answer_type": "poll"}"#,
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": []}"#,
            r#"{"question": "Q?", "answer_type": "text", "possible_answers": ["A"]}"#,
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": ["A", "A"]}"#,
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": ["A", " "]}"#,
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": ["A", ""]}"#,
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": [" A"]}"#,
        ] {
            assert!(Survey::from_json(&survey_json(question)).is_err(), "{question}");
        }
    }

    #[test]
    fn a_poll_may_name_its_free_answer() {
        let with = |label: &str| {
            let json = survey_json(&format!(
                r#"{{"question": "Q?", "answer_type": "poll", "possible_answers": ["No"]{label}}}"#
            ));
            Survey::from_json(&json).map(|survey| survey.questions[0].clone())
        };
        let named = with(r#", "other_label": "Yes, namely:""#).unwrap();
        assert_eq!(named.other_label.as_deref(), Some("Yes, namely:"));
        assert_eq!(named.other_label(), "Yes, namely:");
        // Part of the definition, not of the identity: the id stays the same.
        let plain = with("").unwrap();
        assert_eq!(plain.other_label, None);
        assert_eq!(plain.other_label(), "Other");
        assert_eq!(named.id, plain.id);
    }

    #[test]
    fn invalid_other_labels_are_rejected() {
        for question in [
            r#"{"question": "Q?", "answer_type": "text", "other_label": "Else"}"#,
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": ["A"], "other_label": ""}"#,
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": ["A"], "other_label": " Else"}"#,
        ] {
            assert!(Survey::from_json(&survey_json(question)).is_err(), "{question}");
        }
    }

    #[test]
    fn a_definition_has_no_access_key() {
        // Who may answer is not part of the definition: every survey has a
        // pass phrase, kept in the backend's database.
        let with = |access: &str| {
            Survey::from_json(&format!(
                r#"{{"survey_id": 1, "description": "", {access} "questions": []}}"#
            ))
        };
        assert!(with("").is_ok());
        for access in ["pass_phrase", "name_list"] {
            let err = with(&format!(r#""access": "{access}","#)).unwrap_err();
            assert!(err.to_string().contains("unknown field `access`"), "{err}");
        }
    }

    #[test]
    fn invalid_survey_file_is_rejected() {
        assert!(Survey::from_json("nonsense").is_err());
        assert!(Survey::from_json(r#"{"survey_id": "x", "description": "", "questions": []}"#).is_err());
        assert!(Survey::from_json(r#"{"survey_id": 1, "description": "", "questions": [], "typo": 1}"#).is_err());
    }

    #[test]
    fn correct_answers_make_a_scored_survey() {
        let json = survey_json(
            r#"{"question": "When?", "answer_type": "prediction_date", "correct_answer": {"month": 7, "year": 2028}},
               {"question": "Why?", "answer_type": "text", "correct_answer": "because"},
               {"question": "Which?", "answer_type": "poll", "possible_answers": ["A", "B"], "correct_answer": "B"}"#,
        );
        let survey = Survey::from_json(&json).unwrap();
        assert!(survey.scored);
        let correct: Vec<_> = survey.questions.iter().map(|q| q.correct_answer.clone()).collect();
        assert_eq!(
            correct,
            [
                Some(Answer::PredictionDate {
                    day: None,
                    month: 7,
                    year: 2028
                }),
                Some(Answer::Text("because".into())),
                Some(Answer::Poll("B".into())),
            ]
        );

        assert!(!samples::starship().scored);
        assert!(!samples::course().scored);
    }

    #[test]
    fn scored_surveys_need_a_fitting_correct_answer_for_every_question() {
        let text = |answer: &str| format!(r#"{{"question": "Q?", "answer_type": "text", {answer}}}"#);
        let blank = r#"{"question": "Blank", "answer_type": "text"}"#;
        for questions in [
            // Not every question has one.
            format!("{}, {blank}", text(r#""correct_answer": "x""#)),
            // Of the wrong shape for the kind.
            text(r#""correct_answer": {"month": 1, "year": 2030}"#),
            r#"{"question": "Q?", "answer_type": "prediction_date", "correct_answer": "soon"}"#.into(),
            // Invalid answers.
            text(r#""correct_answer": "  ""#),
            r#"{"question": "Q?", "answer_type": "prediction_date", "correct_answer": {"month": 13, "year": 2030}}"#
                .into(),
            r#"{"question": "Q?", "answer_type": "prediction_date", "correct_answer": {"month": 1, "year": 1999}}"#
                .into(),
            // Not one of the options.
            r#"{"question": "Q?", "answer_type": "poll", "possible_answers": ["A"], "correct_answer": "B"}"#.into(),
        ] {
            assert!(Survey::from_json(&survey_json(&questions)).is_err(), "{questions}");
        }
    }
}
