use actix_web::http::{StatusCode, header};
use actix_web::{HttpRequest, HttpResponse, ResponseError, get, post, web};
use log::error;
use prono::{Error, Repo};
use prono_api::{API_KEY, DEVICE_ID_HEADER, Submission};

/// Registers the REST endpoints; their paths are [`prono_api::answers_path`]
/// and [`prono_api::user_path`].
pub fn routes(config: &mut web::ServiceConfig) {
    config.service(submit_answers).service(get_answers).service(get_user);
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

#[get("/api/surveys/{survey_id}/answers")]
async fn get_answers(repo: web::Data<dyn Repo>, survey_id: web::Path<u64>) -> Result<HttpResponse, ApiError> {
    let results = prono::results(repo.get_ref(), *survey_id).await?;
    Ok(HttpResponse::Ok().json(results))
}

/// Whether the user `name` already filled in a survey: a JSON `bool`, and
/// nothing more. Same API key and device id as a submission.
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
                Error::Invalid(_) | Error::InvalidDeviceId => StatusCode::BAD_REQUEST,
                Error::DeviceMismatch | Error::DeviceInUse => StatusCode::FORBIDDEN,
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
    use prono_api::{Answer, Results};

    use super::*;

    const SURVEY_ID: u64 = 1;

    fn submission(user: &str) -> Submission {
        let survey = prono_api::survey(SURVEY_ID).unwrap();
        Submission::new(user, survey, &survey.empty_answers())
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
        let repo: web::Data<dyn Repo> = web::Data::from(Arc::new(MemoryRepo::default()) as Arc<dyn Repo>);
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
        let survey = prono_api::survey(SURVEY_ID).unwrap();
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
    async fn user_check_answers_yes_or_no() {
        let (statuses, body) = run(vec![
            get_user("alice", Some("dev-2"), API_KEY),
            post(SURVEY_ID, Some("dev-1"), API_KEY, &submission("alice")),
            get_user("alice", Some("dev-2"), API_KEY),
        ])
        .await;
        assert_eq!(statuses, [StatusCode::OK; 3]);
        assert_eq!(body, "true");

        let (_, body) = run(vec![get_user("alice", Some("dev-2"), API_KEY)]).await;
        assert_eq!(body, "false");
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
