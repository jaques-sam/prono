//! The survey definition: the JSON an author uploads to the backend, and how
//! it becomes a [`Survey`].
//!
//! ```json
//! {
//!     "survey_id": 1,
//!     "description": "What the survey is about",
//!     "link": "https://example.com",
//!     "access": "pass_phrase",
//!     "questions": [
//!         { "question": "When?", "answer_type": "prediction_date" },
//!         { "question": "Why?", "answer_type": "text" },
//!         { "question": "Which?", "answer_type": "poll", "possible_answers": ["A", "B"] }
//!     ]
//! }
//! ```
//!
//! `link` is optional. `access` is required: `pass_phrase` or `name_list`.
//! `possible_answers` is for `poll` questions only.

use std::collections::HashSet;

use serde::Deserialize;
use uuid::Uuid;

use crate::{Access, AnswerKind, Question, Survey};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SurveyFile {
    survey_id: u64,
    description: String,
    #[serde(default)]
    link: Option<String>,
    access: Access,
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
        Ok(())
    }
}

impl Survey {
    /// Reads a survey definition (see the [module](self) docs).
    ///
    /// # Errors
    ///
    /// If `json` is no valid definition: malformed JSON, an unknown or
    /// missing key (also a missing `access`), or bad `possible_answers`.
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        let file: SurveyFile = serde_json::from_str(json)?;
        file.questions.iter().try_for_each(QuestionFile::check)?;
        let questions = file
            .questions
            .into_iter()
            .map(|q| Question {
                id: question_id(&q.question, q.answer_type),
                text: q.question,
                kind: q.answer_type,
                possible_answers: q.possible_answers,
            })
            .collect();
        Ok(Survey {
            id: file.survey_id,
            description: file.description,
            link: file.link,
            access: file.access,
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
        assert_eq!(samples::course().questions.len(), 2);
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
        format!(r#"{{"survey_id": 1, "description": "", "access": "name_list", "questions": [{question}]}}"#)
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
    fn access_is_required_and_cannot_be_open() {
        let with_access = |access: &str| {
            Survey::from_json(&format!(
                r#"{{"survey_id": 1, "description": "", {access} "questions": []}}"#
            ))
        };
        assert!(with_access("").is_err(), "no access given");
        assert!(with_access(r#""access": null,"#).is_err());
        assert!(with_access(r#""access": "open","#).is_err());
        assert!(with_access(r#""access": "none","#).is_err());
        assert!(with_access(r#""access": "secret","#).is_err());
        assert_eq!(
            with_access(r#""access": "name_list","#).unwrap().access,
            Access::NameList
        );
        assert_eq!(
            with_access(r#""access": "pass_phrase","#).unwrap().access,
            Access::PassPhrase
        );
        assert_eq!(samples::starship().access, Access::PassPhrase);
        assert_eq!(samples::course().access, Access::NameList);
    }

    #[test]
    fn invalid_survey_file_is_rejected() {
        assert!(Survey::from_json("nonsense").is_err());
        assert!(
            Survey::from_json(r#"{"survey_id": "x", "description": "", "access": "name_list", "questions": []}"#)
                .is_err()
        );
        assert!(
            Survey::from_json(
                r#"{"survey_id": 1, "description": "", "access": "name_list", "questions": [], "typo": 1}"#
            )
            .is_err()
        );
    }
}
