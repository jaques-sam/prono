use std::sync::{Mutex, MutexGuard};

use async_trait::async_trait;
use prono_api::{Answer, Question, UserAnswer};
use uuid::Uuid;

use crate::{Error, Repo, Result};

/// An in-memory [`Repo`], for tests.
#[derive(Default)]
pub struct MemoryRepo {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// `(user, device id)`
    users: Vec<(String, String)>,
    /// `(user, question id, answer)`
    answers: Vec<(String, Uuid, Answer)>,
}

impl MemoryRepo {
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[async_trait]
impl Repo for MemoryRepo {
    async fn device_of(&self, user: &str) -> Result<Option<String>> {
        Ok(self
            .state()
            .users
            .iter()
            .find(|(u, _)| u == user)
            .map(|(_, d)| d.clone()))
    }

    async fn user_of(&self, device_id: &str) -> Result<Option<String>> {
        Ok(self
            .state()
            .users
            .iter()
            .find(|(_, d)| d == device_id)
            .map(|(u, _)| u.clone()))
    }

    async fn save_answers(&self, user: &str, device_id: &str, answers: &[(Uuid, Answer)]) -> Result<()> {
        let mut state = self.state();
        let answered = |question_id: &Uuid| state.answers.iter().any(|(u, q, _)| u == user && q == question_id);
        if answers.iter().any(|(question_id, _)| answered(question_id)) {
            return Err(Error::AnswerExists);
        }
        if !state.users.iter().any(|(u, _)| u == user) {
            state.users.push((user.to_string(), device_id.to_string()));
        }
        for (question_id, answer) in answers {
            state.answers.push((user.to_string(), *question_id, answer.clone()));
        }
        Ok(())
    }

    async fn answers(&self, question: &Question) -> Result<Vec<UserAnswer>> {
        Ok(self
            .state()
            .answers
            .iter()
            .filter(|(_, q, _)| *q == question.id)
            .map(|(user, _, answer)| UserAnswer {
                user: user.clone(),
                answer: answer.clone(),
            })
            .collect())
    }

    async fn users(&self) -> Result<Vec<String>> {
        Ok(self.state().users.iter().map(|(u, _)| u.clone()).collect())
    }

    async fn delete_user(&self, user: &str) -> Result<()> {
        let mut state = self.state();
        state.users.retain(|(u, _)| u != user);
        state.answers.retain(|(u, _, _)| u != user);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use prono_api::AnswerKind;

    use super::*;

    const Q1: Uuid = Uuid::from_u128(1);
    const Q2: Uuid = Uuid::from_u128(2);

    fn question(id: Uuid) -> Question {
        Question {
            id,
            text: String::new(),
            kind: AnswerKind::Text,
            possible_answers: Vec::new(),
        }
    }

    fn text(s: &str) -> Answer {
        Answer::Text(s.into())
    }

    #[tokio::test]
    async fn saves_user_and_answers() {
        let repo = MemoryRepo::default();
        repo.save_answers("alice", "dev-1", &[(Q1, text("a"))]).await.unwrap();

        assert_eq!(repo.device_of("alice").await.unwrap().as_deref(), Some("dev-1"));
        assert_eq!(repo.user_of("dev-1").await.unwrap().as_deref(), Some("alice"));
        assert_eq!(repo.users().await.unwrap(), ["alice"]);
        let answers = repo.answers(&question(Q1)).await.unwrap();
        assert_eq!(
            answers,
            [UserAnswer {
                user: "alice".into(),
                answer: text("a")
            }]
        );
    }

    #[tokio::test]
    async fn rejected_batch_stores_nothing() {
        let repo = MemoryRepo::default();
        repo.save_answers("alice", "dev-1", &[(Q2, text("a"))]).await.unwrap();

        let batch = [(Q1, text("b")), (Q2, text("c"))];
        let err = repo.save_answers("alice", "dev-1", &batch).await.unwrap_err();
        assert_eq!(err, Error::AnswerExists);
        assert_eq!(repo.answers(&question(Q1)).await.unwrap(), Vec::<UserAnswer>::new());
    }

    #[tokio::test]
    async fn delete_user_removes_answers() {
        let repo = MemoryRepo::default();
        repo.save_answers("alice", "dev-1", &[(Q1, text("a"))]).await.unwrap();
        repo.delete_user("alice").await.unwrap();

        assert_eq!(repo.users().await.unwrap(), Vec::<String>::new());
        assert_eq!(repo.answers(&question(Q1)).await.unwrap(), Vec::<UserAnswer>::new());
        assert!(repo.user_of("dev-1").await.unwrap().is_none());
    }
}
