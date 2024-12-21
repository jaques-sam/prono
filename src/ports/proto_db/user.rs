#![allow(dead_code)]

use uuid::Uuid;

use super::DbAnswer;

pub struct DbUser {
    id: Uuid,
    name: String,
    email: String,
    answers: Vec<DbAnswer>,
}
