mod rest;

use std::io;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{App, HttpServer, http::header, web};
use log::info;
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
async fn main() -> io::Result<()> {
    env_logger::init();

    let config = DbConfig::load().map_err(io::Error::other)?;
    let repo = MysqlRepo::connect(&config).await.map_err(io::Error::other)?;
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
    .bind(ADDRESS)?
    .run()
    .await
}
