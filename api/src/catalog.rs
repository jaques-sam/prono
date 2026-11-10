use std::sync::LazyLock;

use serde::Deserialize;
use uuid::Uuid;

use crate::{AnswerKind, Question, Survey};

/// Every survey, compiled into each binary. To add a questionnaire, drop a
/// JSON file in `api/surveys/` and list it here.
const SURVEY_FILES: &[&str] = &[include_str!("../surveys/spacex_starship.json")];

static SURVEYS: LazyLock<Vec<Survey>> = LazyLock::new(|| {
    SURVEY_FILES
        .iter()
        .map(|json| parse(json).unwrap_or_else(|err| panic!("invalid survey file: {err}")))
        .collect()
});

/// All known surveys.
#[must_use]
pub fn surveys() -> &'static [Survey] {
    &SURVEYS
}

/// The survey with id `id`, if it exists.
#[must_use]
pub fn survey(id: u64) -> Option<&'static Survey> {
    surveys().iter().find(|survey| survey.id == id)
}

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
}

fn parse(json: &str) -> serde_json::Result<Survey> {
    let file: SurveyFile = serde_json::from_str(json)?;
    Ok(Survey {
        id: file.survey_id,
        description: file.description,
        link: file.link,
        questions: file
            .questions
            .into_iter()
            .map(|q| Question {
                id: question_id(&q.question, q.answer_type),
                text: q.question,
                kind: q.answer_type,
            })
            .collect(),
    })
}

/// Stored answers are keyed by this id, so it must never change for an
/// existing question: it hashes the question text and the answer kind.
fn question_id(text: &str, kind: AnswerKind) -> String {
    let kind = match kind {
        AnswerKind::Text => "Text",
        AnswerKind::PredictionDate => "PredictionDate",
    };
    Uuid::new_v5(&Uuid::NAMESPACE_DNS, format!("{text}{kind}").as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_survey_file_parses() {
        assert_eq!(surveys().len(), SURVEY_FILES.len());
        assert!(surveys().iter().all(|s| !s.questions.is_empty()));
    }

    #[test]
    fn survey_and_question_ids_are_unique() {
        let survey_ids: HashSet<_> = surveys().iter().map(|s| s.id).collect();
        assert_eq!(survey_ids.len(), surveys().len());

        let question_ids: Vec<_> = surveys().iter().flat_map(|s| &s.questions).map(|q| &q.id).collect();
        let unique: HashSet<_> = question_ids.iter().collect();
        assert_eq!(unique.len(), question_ids.len());
    }

    #[test]
    fn existing_question_ids_are_stable() {
        // Answers already stored in the database reference these ids.
        let ids: Vec<_> = survey(1).unwrap().questions.iter().map(|q| q.id.as_str()).collect();
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
    }

    #[test]
    fn unknown_survey_is_none() {
        assert!(survey(u64::MAX).is_none());
    }

    #[test]
    fn invalid_survey_file_is_rejected() {
        assert!(parse(r#"{"survey_id": "x", "description": "", "questions": []}"#).is_err());
        assert!(parse(r#"{"survey_id": 1, "description": "", "questions": [], "typo": 1}"#).is_err());
    }
}
