use std::sync::Arc;

use prono::repo;

use crate::BackendResult;

pub struct SurveyService {
    db: Arc<dyn repo::Surveys + Send + Sync>,
    devices: Arc<dyn repo::DeviceRegistry + Send + Sync>,
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
    pub fn new(db: Arc<dyn repo::Surveys + Send + Sync>, devices: Arc<dyn repo::DeviceRegistry + Send + Sync>) -> Self {
        Self { db, devices }
    }

    #[must_use]
    pub fn empty_survey(&self) -> prono_api::Survey {
        prono::empty_survey()
    }

    /// # Errors
    ///
    /// Returns an error if the device verification fails, the answer already exists,
    /// or if a repository error occurs.
    pub async fn add_answer(
        &self,
        user: &str,
        question_id: String,
        answer: prono_api::Answer,
        device_id: &str,
    ) -> BackendResult<()> {
        if !self.devices.verify_device(user, device_id).await? {
            return Err(crate::Error::DeviceMismatch);
        }
        self.devices.register_device(user, device_id).await?;
        self.db
            .add_answer(user, question_id, api_answer_to_repo(answer))
            .await?;
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
    async fn test_add_and_retrieve_answer() {
        let service = make_service().await;
        let survey = service.empty_survey();
        let question_id = survey.questions[0].id.clone();

        let answer = prono_api::Answer::Text("test answer".to_string());
        service
            .add_answer("testuser", question_id.clone(), answer, "device-1")
            .await
            .unwrap();

        let all = service.all_answers(question_id).await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "testuser");
    }

    #[tokio::test]
    async fn test_add_duplicate_answer_fails() {
        let service = make_service().await;
        let survey = service.empty_survey();
        let question_id = survey.questions[0].id.clone();

        let answer = prono_api::Answer::Text("answer".to_string());
        service
            .add_answer("user1", question_id.clone(), answer.clone(), "device-1")
            .await
            .unwrap();

        let result = service.add_answer("user1", question_id, answer, "device-1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_answer_device_mismatch() {
        let service = make_service().await;
        let survey = service.empty_survey();
        let q1 = survey.questions[0].id.clone();
        let q2 = survey
            .questions
            .get(1)
            .map_or_else(|| "q2".to_string(), |q| q.id.clone());

        let answer = prono_api::Answer::Text("answer".to_string());
        service
            .add_answer("user1", q1, answer.clone(), "device-1")
            .await
            .unwrap();

        let result = service.add_answer("user1", q2, answer, "device-2").await;
        assert!(result.is_err());
    }
}
