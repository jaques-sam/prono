#![allow(dead_code)]

#[derive(Debug, PartialEq, Eq)]
struct Dbsurvey<A> {
    questions: Vec<DbQuestion<A>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct DbQuestion<A> {
    id: u16,
    question: String,
    answers: A,
}
