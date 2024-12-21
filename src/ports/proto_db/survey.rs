#![allow(dead_code)]

use uuid::Uuid;

use super::DbAnswer;

struct DbUserResponses {
    user_id: Uuid,
    responses: Vec<DbSurveyResponse>,
}

#[derive(Debug, PartialEq, Eq)]
struct DbSurveyResponse {
    survey_id: u64,
    answers: Vec<DbAnswerResponse>,
}

#[derive(Debug, PartialEq, Eq)]
struct DbAnswerResponse {
    question_id: u64,
    answer: DbAnswer,
}
