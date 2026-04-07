use actix_web::{HttpRequest, HttpResponse, get, post, web};
use log::info;
use serde::Deserialize;

use crate::BackendResult;
use crate::Error;
use crate::use_cases::*;

#[derive(Deserialize)]
pub struct AddUserRequest {
    pub user: String,
}

#[derive(Deserialize)]
pub struct AddAnswerRequest {
    pub user: String,
    pub question_id: String,
    pub answer: prono_api::Answer,
}

#[get("/api/survey")]
pub async fn get_survey(service: web::Data<SurveyService>) -> HttpResponse {
    let survey = service.empty_survey();
    HttpResponse::Ok().json(survey)
}

#[post("/api/user")]
pub async fn add_user(
    service: web::Data<SurveyService>,
    body: web::Json<AddUserRequest>,
    req: HttpRequest,
) -> BackendResult<HttpResponse> {
    let device_id = req
        .headers()
        .get("X-Device-Id")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::DeviceMismatch)?;

    info!(
        "/api/user called for user '{}' with device id '{}'",
        body.user, device_id
    );
    service.add_user(&body.user, device_id).await?;

    Ok(HttpResponse::Ok().finish())
}

#[post("/api/survey/answer")]
pub async fn add_answer(
    service: web::Data<SurveyService>,
    body: web::Json<AddAnswerRequest>,
) -> BackendResult<HttpResponse> {
    let body = body.into_inner();

    info!(
        "/api/survey/answer called for user {}: Q:{}, A:{:?}",
        body.user, body.question_id, body.answer
    );
    service.add_answer(&body.user, body.question_id, body.answer).await?;
    Ok(HttpResponse::Ok().finish())
}

#[get("/api/survey/response/{user}/{survey_id}")]
pub async fn get_response(service: web::Data<SurveyService>, path: web::Path<(String, u64)>) -> HttpResponse {
    let (user, survey_id) = path.into_inner();
    match service.response(&user, survey_id).await {
        Some(survey) => HttpResponse::Ok().json(survey),
        None => HttpResponse::NotFound().finish(),
    }
}

#[get("/api/survey/answers/{question_id}")]
pub async fn get_all_answers(service: web::Data<SurveyService>, path: web::Path<String>) -> HttpResponse {
    let question_id = path.into_inner();
    let answers = service.all_answers(question_id).await;
    HttpResponse::Ok().json(answers)
}
