use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::Deserialize;

use crate::BackendResult;
use crate::use_cases::*;

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
        .unwrap_or("");
    let body = body.into_inner();
    service
        .add_answer(&body.user, body.question_id, body.answer, device_id)
        .await?;
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
