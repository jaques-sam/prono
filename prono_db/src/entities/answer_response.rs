#![allow(dead_code)]

#[derive(Debug, PartialEq, Eq)]
struct AnswerResponse {
    user_id: u64,
    question_id: u64,
    answer: String,
}
