use crate::{Error, PronoResult};
use async_trait::async_trait;
use log::{error, info};

use crate::repo::{self, Answer, Survey};
use std::collections::HashMap;
use tokio::sync::Mutex;

pub struct FakeRepo {
    surveys: Mutex<HashMap<String, Survey>>,
}

#[async_trait]
impl repo::Db for FakeRepo {
    type Config = ();

    async fn init(_config: Self::Config) -> PronoResult<Self> {
        info!("Initializing fake database...");

        Ok(Self {
            surveys: Mutex::new(HashMap::new()),
        })
    }
}

#[async_trait]
impl repo::Surveys for FakeRepo {
    async fn answer(&self, user: &str, question_id: String) -> Option<Answer> {
        info!("Fetching answer from user {user} for Q:{question_id}");
        self.surveys
            .lock()
            .await
            .get(user)?
            .questions
            .iter()
            .find_map(|q| (q.id == question_id).then_some(q.answer.clone()))
    }

    async fn response(&self, user: &str, survey_id: u64) -> Option<Survey> {
        info!("Fetching survey [{survey_id}] response from user {user}");
        self.surveys.lock().await.get(user).cloned()
    }

    async fn add_answer(&self, user: &str, question_id: String, answer: Answer) -> PronoResult<()> {
        let mut surveys = self.surveys.lock().await;
        let user_surveys = surveys.entry(user.to_string()).or_insert_with(|| Survey {
            questions: vec![],
            id: 0,
            description: None,
        });

        if user_surveys.questions.iter().any(|q| q.id == question_id) {
            error!("User {user} already answered Q:{question_id}");
            return Err(Error::AnswerExists);
        }

        info!("Adding answer from user {user} for Q:{question_id}");
        user_surveys.questions.push(crate::repo::Question {
            id: question_id,
            answer,
        });

        Ok(())
    }

    async fn all_answers(&self, question_id: String) -> Vec<(String, Answer)> {
        info!("Fetching all answers for Q:{question_id}");
        self.surveys
            .lock()
            .await
            .iter()
            .flat_map(|(user, survey)| {
                survey
                    .questions
                    .iter()
                    .filter(|q| q.id == question_id)
                    .map(move |q| (user.clone(), q.answer.clone()))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::repo::{Db, Surveys};

    use super::*;

    fn setup() -> FakeRepo {
        FakeRepo {
            surveys: Mutex::new(HashMap::new()),
        }
    }

    #[tokio::test]
    async fn test_init() {
        let repo = FakeRepo::init(()).await;
        assert!(repo.is_ok());
    }

    #[tokio::test]
    async fn test_answer_not_found() {
        let repo = setup();
        let result = repo.answer("user1", "q1".to_string()).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_response_not_found() {
        let repo = setup();
        let result = repo.response("user1", 1).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_add_answer_user_not_exists() {
        let repo = setup();
        let answer = Answer::default();
        let result = repo.add_answer("user1", "q1".to_string(), answer.clone()).await;
        assert!(result.is_ok());

        // Verify the answer was stored
        let stored_answer = repo.answer("user1", "q1".to_string()).await;
        assert_eq!(stored_answer, Some(answer));
    }

    #[tokio::test]
    async fn test_all_answers_empty() {
        let repo = setup();
        let results = repo.all_answers("q1".to_string()).await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_answer_found() {
        let repo = setup();
        let answer = Answer::default();
        let question = crate::repo::Question {
            id: "q1".to_string(),
            answer: answer.clone(),
        };
        let survey = Survey {
            questions: vec![question],
            ..Survey::default()
        };
        repo.surveys.lock().await.insert("user1".to_string(), survey);

        let result = repo.answer("user1", "q1".to_string()).await;
        assert_eq!(result, Some(answer));
    }

    #[tokio::test]
    async fn test_response_found() {
        let repo = setup();
        let survey = Survey {
            questions: vec![],
            ..Survey::default()
        };
        repo.surveys.lock().await.insert("user1".to_string(), survey.clone());

        let result = repo.response("user1", 1).await;
        assert_eq!(result, Some(survey));
    }

    #[tokio::test]
    async fn test_that_a_user_cannot_update_the_same_question() {
        let repo = setup();
        let question = crate::repo::Question {
            id: "q1".to_string(),
            ..crate::repo::Question::default()
        };
        let survey = Survey {
            questions: vec![question],
            ..Survey::default()
        };
        repo.surveys.lock().await.insert("user1".to_string(), survey);
        let result = repo.add_answer("user1", "q1".to_string(), Answer::default()).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::AnswerExists);
    }

    #[tokio::test]
    async fn test_add_new_question_to_existing_user() {
        let repo = setup();
        let question = crate::repo::Question {
            id: "q1".to_string(),
            ..crate::repo::Question::default()
        };
        let survey = Survey {
            questions: vec![question],
            ..Survey::default()
        };
        repo.surveys.lock().await.insert("user1".to_string(), survey);

        let new_answer = Answer::default();
        let result = repo.add_answer("user1", "q2".to_string(), new_answer.clone()).await;
        assert!(result.is_ok());

        // Verify both questions are stored
        let answer1 = repo.answer("user1", "q1".to_string()).await;
        let answer2 = repo.answer("user1", "q2".to_string()).await;
        assert!(answer1.is_some());
        assert_eq!(answer2, Some(new_answer));
    }

    #[tokio::test]
    async fn test_all_answers_multiple_users() {
        let repo = setup();

        let question_id = "q".to_string();

        let q1 = crate::repo::Question {
            id: question_id.clone(),
            ..crate::repo::Question::default()
        };
        let q2 = crate::repo::Question {
            id: question_id.clone(),
            ..crate::repo::Question::default()
        };

        repo.surveys.lock().await.insert(
            "user1".to_string(),
            Survey {
                questions: vec![q1],
                ..Survey::default()
            },
        );
        repo.surveys.lock().await.insert(
            "user2".to_string(),
            Survey {
                questions: vec![q2],
                ..Survey::default()
            },
        );

        let results = repo.all_answers(question_id).await;
        assert_eq!(results.len(), 2);
    }
}
