use actix_web::{HttpRequest, HttpResponse, get, post, web};
use log::info;
use serde::Deserialize;

use crate::BackendResult;
use crate::Error;
use crate::use_cases::*;

#[derive(Deserialize)]
pub struct AddAnswerRequest {
    pub user: String,
    pub question_id: String,
    pub answer: prono_api::Answer,
}

#[derive(Deserialize)]
pub struct AddAnswersRequest {
    pub user: String,
    pub answers: Vec<(String, prono_api::Answer)>,
}

#[get("/api/survey")]
pub async fn get_survey(service: web::Data<SurveyService>) -> HttpResponse {
    let survey = service.empty_survey();
    HttpResponse::Ok().json(survey)
}

#[post("/api/survey/answer")]
pub async fn add_answer(
    service: web::Data<SurveyService>,
    body: web::Json<AddAnswerRequest>,
    req: HttpRequest,
) -> BackendResult<HttpResponse> {
    let device_id = req
        .headers()
        .get("X-Device-Id")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::MissingDeviceId)?;
    let body = body.into_inner();

    info!(
        "/api/survey/answer (deprecated - use /answers) called for user '{}' (device='{device_id}'): Q:{}, A:{:?}",
        body.user, body.question_id, body.answer
    );
    service
        .add_answers(&body.user, device_id, vec![(body.question_id, body.answer)])
        .await?;
    Ok(HttpResponse::Ok().finish())
}

#[post("/api/survey/answers")]
pub async fn add_answers(
    service: web::Data<SurveyService>,
    body: web::Json<AddAnswersRequest>,
    req: HttpRequest,
) -> BackendResult<HttpResponse> {
    let device_id = req
        .headers()
        .get("X-Device-Id")
        .and_then(|v| v.to_str().ok())
        .ok_or(Error::MissingDeviceId)?;
    let body = body.into_inner();

    info!(
        "/api/survey/answers called for user '{}' (device='{device_id}'): {} answers",
        body.user,
        body.answers.len()
    );
    service.add_answers(&body.user, device_id, body.answers).await?;
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
