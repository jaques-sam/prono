use actix_web::http::{StatusCode, header};
use actix_web::{HttpRequest, HttpResponse, ResponseError, get, post, web};
use log::error;
use prono::{Error, Repo};
use prono_api::{API_KEY, DEVICE_ID_HEADER, Submission};

/// Registers the REST endpoints; their paths are [`prono_api::answers_path`],
/// [`prono_api::surveys_path`], [`prono_api::scores_path`] and
/// [`prono_api::user_path`].
pub fn routes(config: &mut web::ServiceConfig) {
    config
        .service(submit_answers)
        .service(open_surveys)
        .service(get_answers)
        .service(get_scores)
        .service(get_user);
}

#[post("/api/surveys/{survey_id}/answers")]
async fn submit_answers(
    repo: web::Data<dyn Repo>,
    survey_id: web::Path<u64>,
    submission: web::Json<Submission>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let device_id = authorized_device(&req)?;
    prono::submit(repo.get_ref(), *survey_id, device_id, &submission).await?;
    Ok(HttpResponse::Ok().finish())
}

/// The surveys a pass phrase opens, as a JSON list of [`prono_api::Survey`]s:
/// `200`, or `403` when it opens none. Same API key and device id as a
/// submission.
#[post("/api/surveys")]
async fn open_surveys(
    repo: web::Data<dyn Repo>,
    pass_phrase: web::Json<Option<String>>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    authorized_device(&req)?;
    let surveys = prono::open_surveys(repo.get_ref(), pass_phrase.as_deref()).await?;
    Ok(HttpResponse::Ok().json(surveys))
}

#[get("/api/surveys/{survey_id}/answers")]
async fn get_answers(repo: web::Data<dyn Repo>, survey_id: web::Path<u64>) -> Result<HttpResponse, ApiError> {
    let results = prono::results(repo.get_ref(), *survey_id).await?;
    Ok(HttpResponse::Ok().json(results))
}

/// The scores of a scored survey, best first, as a JSON list of
/// [`prono_api::Score`]s.
#[get("/api/surveys/{survey_id}/scores")]
async fn get_scores(repo: web::Data<dyn Repo>, survey_id: web::Path<u64>) -> Result<HttpResponse, ApiError> {
    let scores = prono::scores(repo.get_ref(), *survey_id).await?;
    Ok(HttpResponse::Ok().json(scores))
}

/// What the app may know about the user `name`: whether they already filled
/// in a survey, as a JSON `bool`, and nothing more. Same API key and device id
/// as a submission.
#[get("/api/users/{name}")]
async fn get_user(
    repo: web::Data<dyn Repo>,
    name: web::Path<String>,
    req: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    let device_id = authorized_device(&req)?;
    let answered = prono::has_answered(repo.get_ref(), device_id, &name).await?;
    Ok(HttpResponse::Ok().json(answered))
}

/// Checks the API key and returns the id of the calling device.
fn authorized_device(req: &HttpRequest) -> Result<&str, ApiError> {
    if header_value(req, header::AUTHORIZATION.as_str()).and_then(|v| v.strip_prefix("Bearer ")) != Some(API_KEY) {
        return Err(ApiError::Unauthorized);
    }
    header_value(req, DEVICE_ID_HEADER).ok_or(ApiError::MissingDeviceId)
}

fn header_value<'a>(req: &'a HttpRequest, name: &str) -> Option<&'a str> {
    req.headers().get(name).and_then(|v| v.to_str().ok())
}

/// A failed request. The response body is a JSON string the app shows to the
/// participant.
#[derive(Debug, thiserror::Error)]
enum ApiError {
    #[error("Invalid or missing API key")]
    Unauthorized,
    #[error("Missing {DEVICE_ID_HEADER} header")]
    MissingDeviceId,
    #[error(transparent)]
    Prono(#[from] Error),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
            ApiError::MissingDeviceId => StatusCode::BAD_REQUEST,
            ApiError::Prono(err) => match err {
                Error::Invalid(_) | Error::InvalidDeviceId | Error::InvalidSurvey(_) => StatusCode::BAD_REQUEST,
                Error::DeviceMismatch | Error::DeviceInUse | Error::NotListed | Error::WrongPassPhrase => {
                    StatusCode::FORBIDDEN
                }
                Error::AnswerExists => StatusCode::CONFLICT,
                Error::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }

    fn error_response(&self) -> HttpResponse {
        let message = if let ApiError::Prono(Error::Repository(_)) = self {
            // Details stay in the log, not in front of participants.
            error!("{self}");
            "Internal server error".to_string()
        } else {
            self.to_string()
        };
        HttpResponse::build(self.status_code()).json(message)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_web::{App, test as actix_test};
    use prono::MemoryRepo;
    use prono_api::{Answer, Results, Score, Survey, samples};

    use super::*;

    const SURVEY_ID: u64 = 1;
    const COURSE_ID: u64 = 2;
    /// The pass phrase of the Starship survey and the quiz.
    const PASS_PHRASE: &str = "open sesame";
    const COURSE_PASS_PHRASE: &str = "rust rocks";
    const QUIZ_ID: u64 = 3;
    const QUIZ: &str = r#"{
        "survey_id": 3, "title": "Quiz", "description": "A quiz",
        "questions": [{"question": "Capital of Belgium", "answer_type": "text", "correct_answer": "Brussels"}]
    }"#;

    fn submission(user: &str) -> Submission {
        let survey = samples::starship();
        Submission::new(user, &survey, &survey.empty_answers()).with_pass_phrase(PASS_PHRASE)
    }

    fn post(survey_id: u64, device_id: Option<&str>, key: &str, body: &Submission) -> actix_test::TestRequest {
        let mut req = actix_test::TestRequest::post()
            .uri(&prono_api::answers_path(survey_id))
            .insert_header((header::AUTHORIZATION, format!("Bearer {key}")))
            .set_json(body);
        if let Some(device_id) = device_id {
            req = req.insert_header((DEVICE_ID_HEADER, device_id));
        }
        req
    }

    /// Sends each request in turn to one backend and returns the statuses
    /// and the last response body.
    async fn run(requests: Vec<actix_test::TestRequest>) -> (Vec<StatusCode>, String) {
        let repo = MemoryRepo::with_surveys([samples::STARSHIP_JSON, samples::COURSE_JSON, QUIZ]);
        repo.set_pass_phrase(SURVEY_ID, PASS_PHRASE).await.unwrap();
        repo.set_pass_phrase(COURSE_ID, COURSE_PASS_PHRASE).await.unwrap();
        repo.allow_user(COURSE_ID, "Ada", "Lovelace").await.unwrap();
        repo.set_pass_phrase(QUIZ_ID, PASS_PHRASE).await.unwrap();
        let repo: web::Data<dyn Repo> = web::Data::from(Arc::new(repo) as Arc<dyn Repo>);
        let app = actix_test::init_service(App::new().app_data(repo).configure(routes)).await;
        let mut statuses = Vec::new();
        let mut body = String::new();
        for req in requests {
            let resp = actix_test::call_service(&app, req.to_request()).await;
            statuses.push(resp.status());
            body = String::from_utf8(actix_test::read_body(resp).await.to_vec()).unwrap();
        }
        (statuses, body)
    }

    #[actix_web::test]
    async fn submitted_answers_are_returned() {
        let (statuses, body) = run(vec![
            post(SURVEY_ID, Some("dev-1"), API_KEY, &submission("alice")),
            actix_test::TestRequest::get().uri(&prono_api::answers_path(SURVEY_ID)),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::OK, StatusCode::OK]);

        let results: Results = serde_json::from_str(&body).unwrap();
        let survey = samples::starship();
        let first = &results[&survey.questions[0].id];
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].user, "alice");
    }

    #[actix_web::test]
    async fn errors_map_to_statuses_and_messages() {
        let mut wrong_kind = submission("carol");
        wrong_kind.answers[0].1 = Answer::Text("soon".into());

        let (statuses, body) = run(vec![
            post(SURVEY_ID, Some("dev-1"), "wrong key", &submission("alice")),
            post(SURVEY_ID, None, API_KEY, &submission("alice")),
            post(999, Some("dev-1"), API_KEY, &submission("alice")),
            post(SURVEY_ID, Some("dev-1"), API_KEY, &wrong_kind),
            post(SURVEY_ID, Some("dev-1"), API_KEY, &submission("alice")),
            post(SURVEY_ID, Some("dev-2"), API_KEY, &submission("alice")),
            post(SURVEY_ID, Some("dev-1"), API_KEY, &submission("bob")),
            post(SURVEY_ID, Some("dev-1"), API_KEY, &submission("alice")),
        ])
        .await;
        assert_eq!(
            statuses,
            [
                StatusCode::UNAUTHORIZED,
                StatusCode::BAD_REQUEST,
                StatusCode::BAD_REQUEST,
                StatusCode::BAD_REQUEST,
                StatusCode::OK,
                StatusCode::FORBIDDEN,
                StatusCode::FORBIDDEN,
                StatusCode::CONFLICT,
            ]
        );
        assert_eq!(body, "\"You already answered this survey\"");
    }

    #[actix_web::test]
    async fn malformed_question_id_is_a_bad_request() {
        let body = r#"{"user":"alice","answers":[["not-a-uuid",{"Text":"x"}]]}"#;
        let req = actix_test::TestRequest::post()
            .uri(&prono_api::answers_path(SURVEY_ID))
            .insert_header((header::AUTHORIZATION, format!("Bearer {API_KEY}")))
            .insert_header((DEVICE_ID_HEADER, "dev-1"))
            .insert_header((header::CONTENT_TYPE, "application/json"))
            .set_payload(body);
        let (statuses, _) = run(vec![req]).await;
        assert_eq!(statuses, [StatusCode::BAD_REQUEST]);
    }

    fn get_user(name: &str, device_id: Option<&str>, key: &str) -> actix_test::TestRequest {
        let mut req = actix_test::TestRequest::get()
            .uri(&prono_api::user_path(name))
            .insert_header((header::AUTHORIZATION, format!("Bearer {key}")));
        if let Some(device_id) = device_id {
            req = req.insert_header((DEVICE_ID_HEADER, device_id));
        }
        req
    }

    #[actix_web::test]
    async fn user_check_reports_whether_the_name_answered() {
        let (statuses, body) = run(vec![
            post(SURVEY_ID, Some("dev-1"), API_KEY, &submission("alice")),
            get_user("alice", Some("dev-2"), API_KEY),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::OK; 2]);
        assert_eq!(body, "true");

        let (_, body) = run(vec![get_user("alice", Some("dev-2"), API_KEY)]).await;
        assert_eq!(body, "false");
    }

    #[actix_web::test]
    async fn a_survey_needs_its_own_pass_phrase() {
        let survey = samples::course();
        let answers = samples::course_answers();
        let with = |phrase: &str| Submission::new("Ada Lovelace", &survey, &answers).with_pass_phrase(phrase);
        let (statuses, body) = run(vec![
            post(COURSE_ID, Some("dev-1"), API_KEY, &with(PASS_PHRASE)),
            post(COURSE_ID, Some("dev-1"), API_KEY, &with(COURSE_PASS_PHRASE)),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::FORBIDDEN, StatusCode::OK]);
        assert_eq!(body, "");
    }

    #[actix_web::test]
    async fn a_name_list_survey_refuses_a_name_that_is_not_on_its_list() {
        let survey = samples::course();
        let submission =
            Submission::new("Bob Ray", &survey, &samples::course_answers()).with_pass_phrase(COURSE_PASS_PHRASE);
        let (statuses, body) = run(vec![post(COURSE_ID, Some("dev-1"), API_KEY, &submission)]).await;
        assert_eq!(statuses, [StatusCode::FORBIDDEN]);
        assert_eq!(body, "\"You are not on the list for this survey\"");
    }

    #[actix_web::test]
    async fn wrong_pass_phrase_is_forbidden() {
        let survey = samples::starship();
        let wrong = Submission::new("alice", &survey, &survey.empty_answers()).with_pass_phrase("nope");
        let (statuses, body) = run(vec![post(SURVEY_ID, Some("dev-1"), API_KEY, &wrong)]).await;
        assert_eq!(statuses, [StatusCode::FORBIDDEN]);
        assert_eq!(body, "\"Wrong pass phrase\"");
    }

    fn open(device_id: Option<&str>, key: &str, pass_phrase: Option<&str>) -> actix_test::TestRequest {
        let mut req = actix_test::TestRequest::post()
            .uri(prono_api::surveys_path())
            .insert_header((header::AUTHORIZATION, format!("Bearer {key}")))
            .set_json(pass_phrase);
        if let Some(device_id) = device_id {
            req = req.insert_header((DEVICE_ID_HEADER, device_id));
        }
        req
    }

    fn opened(body: &str) -> Vec<u64> {
        let surveys: Vec<Survey> = serde_json::from_str(body).unwrap();
        surveys.iter().map(|survey| survey.id).collect()
    }

    #[actix_web::test]
    async fn a_pass_phrase_opens_the_surveys_that_have_it() {
        let (statuses, body) = run(vec![open(Some("dev-1"), API_KEY, Some(PASS_PHRASE))]).await;
        assert_eq!(statuses, [StatusCode::OK]);
        assert_eq!(opened(&body), [SURVEY_ID, QUIZ_ID]);
        let surveys: Vec<Survey> = serde_json::from_str(&body).unwrap();
        assert_eq!(surveys[0], samples::starship());

        let (_, body) = run(vec![open(Some("dev-1"), API_KEY, Some(COURSE_PASS_PHRASE))]).await;
        assert_eq!(opened(&body), [COURSE_ID]);
    }

    #[actix_web::test]
    async fn a_pass_phrase_that_opens_nothing_is_forbidden() {
        for wrong in [Some("nope"), Some(""), None] {
            let (statuses, body) = run(vec![open(Some("dev-1"), API_KEY, wrong)]).await;
            assert_eq!(statuses, [StatusCode::FORBIDDEN]);
            assert_eq!(body, "\"Wrong pass phrase\"");
        }
    }

    #[actix_web::test]
    async fn opening_surveys_needs_the_api_key_and_a_device_id() {
        let (statuses, body) = run(vec![
            open(Some("dev-1"), "wrong-key", Some(PASS_PHRASE)),
            open(None, API_KEY, Some(PASS_PHRASE)),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::UNAUTHORIZED, StatusCode::BAD_REQUEST]);
        // Not even the existence of a survey is told.
        assert!(!body.contains("Starship"));
    }

    #[actix_web::test]
    async fn scores_rank_the_participants_of_a_scored_survey() {
        let quiz = Survey::from_json(QUIZ).unwrap();
        let answer = |user: &str, text: &str| {
            Submission::new(user, &quiz, &[Answer::Text(text.into())]).with_pass_phrase(PASS_PHRASE)
        };
        let (statuses, body) = run(vec![
            post(QUIZ_ID, Some("dev-1"), API_KEY, &answer("Ann", "Paris")),
            post(QUIZ_ID, Some("dev-2"), API_KEY, &answer("Bob", "Brussels")),
            actix_test::TestRequest::get().uri(&prono_api::scores_path(QUIZ_ID)),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::OK; 3]);
        let scores: Vec<Score> = serde_json::from_str(&body).unwrap();
        let points = |user: &str, points| Score {
            user: user.into(),
            points,
        };
        assert_eq!(scores, [points("Bob", 1), points("Ann", 0)]);
        // The correct answers do not leave the backend, not even in the survey list.
        let (_, body) = run(vec![open(Some("dev-1"), API_KEY, Some(PASS_PHRASE))]).await;
        assert!(!body.contains("Brussels"));
        assert!(body.contains(r#""scored":true"#));
    }

    #[actix_web::test]
    async fn scores_of_a_survey_without_correct_answers_are_a_bad_request() {
        let (statuses, body) = run(vec![
            actix_test::TestRequest::get().uri(&prono_api::scores_path(SURVEY_ID)),
            actix_test::TestRequest::get().uri(&prono_api::scores_path(999)),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::BAD_REQUEST; 2]);
        assert_eq!(body, "\"Unknown survey 999\"");
    }

    #[actix_web::test]
    async fn user_check_handles_special_characters() {
        let name = "Zoë a/b?";
        let (statuses, body) = run(vec![
            post(SURVEY_ID, Some("dev-1"), API_KEY, &submission(name)),
            get_user(name, Some("dev-1"), API_KEY),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::OK; 2]);
        assert_eq!(body, "true");
    }

    #[actix_web::test]
    async fn user_check_follows_the_submission_rules() {
        let (statuses, _) = run(vec![
            get_user("alice", Some("dev-1"), "wrong key"),
            get_user("alice", None, API_KEY),
            get_user("alice", Some(" "), API_KEY),
            get_user(&"x".repeat(100), Some("dev-1"), API_KEY),
        ])
        .await;
        assert_eq!(
            statuses,
            [
                StatusCode::UNAUTHORIZED,
                StatusCode::BAD_REQUEST,
                StatusCode::BAD_REQUEST,
                StatusCode::BAD_REQUEST,
            ]
        );
    }

    #[test]
    fn repository_details_are_not_exposed() {
        let err = ApiError::Prono(Error::Repository("password=hunter2".into()));
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = actix_web::body::MessageBody::try_into_bytes(err.error_response().into_body()).unwrap();
        assert_eq!(body, "\"Internal server error\"");
    }
}
