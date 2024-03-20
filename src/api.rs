use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use log::info;
use std::env;

pub async fn run() -> std::io::Result<()> {
    let web_api_port = env::var("WEB_API_PORT").unwrap_or(String::from("8081"));
    info!("[api] Starting web server on port: {}", web_api_port);
    HttpServer::new(|| App::new().service(ping))
        .bind(format!("localhost:{}", web_api_port))?
        .run()
        .await
}

#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong")
}
