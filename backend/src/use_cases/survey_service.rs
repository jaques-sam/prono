use std::sync::Arc;

use log::error;
use prono::repo;
use prono::submit_answers;

use crate::BackendResult;

pub struct SurveyService {
    db: Arc<dyn repo::Surveys + Send + Sync>,
    users: Arc<dyn repo::Users + Send + Sync>,
}

fn api_answer_to_repo(answer: prono_api::Answer) -> repo::Answer {
    match answer {
        prono_api::Answer::Text(text) => repo::Answer::Text(text),
        prono_api::Answer::PredictionDate { day, month, year } => repo::Answer::PredictionDate { day, month, year },
    }
}

fn repo_answer_to_api(answer: repo::Answer) -> prono_api::Answer {
    match answer {
        repo::Answer::Text(text) => prono_api::Answer::Text(text),
        repo::Answer::PredictionDate { day, month, year } => prono_api::Answer::PredictionDate { day, month, year },
    }
}

fn repo_question_to_api(question: repo::Question) -> prono_api::Question {
    prono_api::Question {
        id: question.id,
        answer: repo_answer_to_api(question.answer),
        text: None,
    }
}

fn repo_survey_to_api(survey: repo::Survey) -> prono_api::Survey {
    prono_api::Survey {
        id: survey.id,
        description: survey.description.unwrap_or_default(),
        questions: survey.questions.into_iter().map(repo_question_to_api).collect(),
    }
}

impl SurveyService {
    pub fn new(db: Arc<dyn repo::Surveys + Send + Sync>, users: Arc<dyn repo::Users + Send + Sync>) -> Self {
        Self { db, users }
    }

    #[must_use]
    pub fn empty_survey(&self) -> prono_api::Survey {
        prono::empty_survey()
    }

    /// # Errors
    ///
    /// Returns `Error::InvalidQuestionId` if the question ID is not found
    pub fn validate_question_id(&self, question_id: &str) -> BackendResult<()> {
        let survey = self.empty_survey();
        if survey.questions.iter().any(|q| q.id == question_id) {
            Ok(())
        } else {
            Err(crate::Error::InvalidQuestionId(format!(
                "Question ID '{question_id}' not found in survey"
            )))
        }
    }

    /// # Errors
    ///
    /// Returns an error if any question id is invalid, if the device id does
    /// not match the registered one (release builds), or if any repository
    /// operation fails.
    pub async fn add_answers(
        &self,
        user: &str,
        #[cfg_attr(debug_assertions, allow(unused_variables))] device_id: &str,
        answers: Vec<(String, prono_api::Answer)>,
    ) -> BackendResult<()> {
        // Validate all question IDs before processing
        for (question_id, _) in &answers {
            if let Err(e) = self.validate_question_id(question_id) {
                error!("add_answers rejected for user='{user}' Q={question_id}: {e}");
                return Err(e);
            }
        }

        // In debug builds, generate a fresh device id so local development
        // ("Survey again" with a different user on the same machine) doesn't
        // get blocked by the device-mismatch check inside the use case.
        #[cfg(debug_assertions)]
        let owned_device_id = uuid::Uuid::new_v4().to_string();
        #[cfg(debug_assertions)]
        let device_id = owned_device_id.as_str();

        let repo_answers: Vec<(String, repo::Answer)> =
            answers.into_iter().map(|(q, a)| (q, api_answer_to_repo(a))).collect();

        submit_answers(&*self.users, &*self.db, user, device_id, repo_answers).await?;
        Ok(())
    }

    pub async fn response(&self, user: &str, survey_id: u64) -> Option<prono_api::Survey> {
        self.db.response(user, survey_id).await.map(repo_survey_to_api)
    }

    pub async fn all_answers(&self, question_id: String) -> Vec<(String, prono_api::Answer)> {
        self.db
            .all_answers(question_id)
            .await
            .into_iter()
            .map(|(user, answer)| (user, repo_answer_to_api(answer)))
            .collect()
    }
}

#[cfg(all(test, debug_assertions))]
mod tests {
    use super::*;

    use prono::fake_db::FakeRepo;
    use prono::repo::Db as _;

    async fn make_service() -> SurveyService {
        let db = Arc::new(FakeRepo::init(()).await.unwrap());
        SurveyService::new(db.clone(), db)
    }

    #[tokio::test]
    async fn test_empty_survey_returns_survey() {
        let service = make_service().await;
        let survey = service.empty_survey();
        assert!(!survey.questions.is_empty());
    }

    #[tokio::test]
    async fn test_add_and_retrieve_answers() {
        let service = make_service().await;
        let survey = service.empty_survey();
        let q1 = survey.questions[0].id.clone();
        let q2 = survey.questions[1].id.clone();

        let answers = vec![
            (q1.clone(), prono_api::Answer::Text("answer1".to_string())),
            (q2.clone(), prono_api::Answer::Text("answer2".to_string())),
        ];
        service.add_answers("testuser", "device-1", answers).await.unwrap();

        let all1 = service.all_answers(q1).await;
        assert_eq!(all1.len(), 1);
        assert_eq!(all1[0].0, "testuser");

        let all2 = service.all_answers(q2).await;
        assert_eq!(all2.len(), 1);
        assert_eq!(all2[0].0, "testuser");
    }

    #[tokio::test]
    async fn test_add_duplicate_answer_fails() {
        let service = make_service().await;
        let survey = service.empty_survey();
        let question_id = survey.questions[0].id.clone();

        let answers = vec![(question_id.clone(), prono_api::Answer::Text("answer".to_string()))];
        service.add_answers("user1", "device-1", answers).await.unwrap();

        let duplicate = vec![(question_id, prono_api::Answer::Text("different".to_string()))];
        let result = service.add_answers("user1", "device-1", duplicate).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_response_after_adding_answers() {
        let service = make_service().await;
        let survey = service.empty_survey();
        let question_id = survey.questions[0].id.clone();

        let answers = vec![(question_id, prono_api::Answer::Text("my answer".to_string()))];
        service.add_answers("user1", "device-1", answers).await.unwrap();

        let response = service.response("user1", 0).await;
        assert!(response.is_some());
        let response = response.unwrap();
        assert_eq!(response.questions.len(), 1);
    }

    #[tokio::test]
    async fn test_response_returns_none_for_unknown_user() {
        let service = make_service().await;
        let response = service.response("nobody", 0).await;
        assert!(response.is_none());
    }
}
