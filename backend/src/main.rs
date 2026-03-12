use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, web};
use log::info;
use prono::ReadConfig;
use prono::repo::Db;

use backend::adapters::rest;
use backend::use_cases::SurveyService;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config_reader = prono::factory::create_config_reader();
    let default_config_path = config_reader.default_config_path();
    let db_config: prono_db::Config = config_reader.read(default_config_path).db.into();

    let db = prono_db::MysqlDb::init(db_config)
        .await
        .expect("Failed to initialize database");

    let service = web::Data::new(SurveyService::new(Arc::new(db)));

    info!("Starting backend server on 0.0.0.0:8081");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("https://jaques-sam.github.io")
            .allowed_origin("http://127.0.0.1:8080")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_header(actix_web::http::header::CONTENT_TYPE)
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(service.clone())
            .service(rest::get_survey)
            .service(rest::add_answer)
            .service(rest::get_response)
            .service(rest::get_all_answers)
    })
    .bind(("0.0.0.0", 8081))?
    .run()
    .await
}
