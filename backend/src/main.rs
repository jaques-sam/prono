mod rest;

use std::process::ExitCode;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http::header, web};
use log::{error, info};
use prono::{DbConfig, MysqlRepo, Repo};

const ADDRESS: (&str, u16) = ("0.0.0.0", 8081);
const ALLOWED_ORIGINS: &[&str] = &[
    "https://jaques-sam.github.io",
    "https://prono-app.samagali.myds.me",
    "http://127.0.0.1:8080",
    "http://localhost:8080",
    "http://127.0.0.1:8888",
    "http://localhost:8888",
];

#[actix_web::main]
async fn main() -> ExitCode {
    env_logger::init();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), String> {
    let config = DbConfig::load()?;
    let repo = MysqlRepo::connect(&config).await.map_err(|err| err.to_string())?;
    let repo: web::Data<dyn Repo> = web::Data::from(Arc::new(repo) as Arc<dyn Repo>);

    info!("Starting backend server on {}:{}", ADDRESS.0, ADDRESS.1);
    HttpServer::new(move || {
        let cors = ALLOWED_ORIGINS
            .iter()
            .fold(Cors::default(), |cors, origin| cors.allowed_origin(origin))
            .allowed_methods(["GET", "POST"])
            .allowed_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
            .allowed_header(prono_api::DEVICE_ID_HEADER)
            .max_age(3600);
        App::new().wrap(cors).app_data(repo.clone()).configure(rest::routes)
    })
    .bind(ADDRESS)
    .map_err(|err| format!("cannot listen on {}:{}: {err}", ADDRESS.0, ADDRESS.1))?
    .run()
    .await
    .map_err(|err| err.to_string())
}
