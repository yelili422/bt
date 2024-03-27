mod rss_api;

use actix_http::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder, ResponseError};
use log::info;

pub use rss_api::*;

pub async fn run() -> std::io::Result<()> {
    info!("[api] Starting web server...");
    HttpServer::new(|| setup_app())
        .bind(("127.0.0.1", 8081))?
        .run()
        .await
}

pub(crate) fn setup_app() -> App<
    impl ServiceFactory<
        ServiceRequest,
        Config = (),
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
        InitError = (),
    >,
> {
    let rss_scope = web::scope("/rss")
        .service(get_rss)
        .service(add_rss)
        .service(delete_rss)
        .service(update_rss);

    App::new()
        .wrap(Logger::default())
        .service(ping)
        .service(rss_scope)
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Internal Error: {0}")]
    InternalError(#[from] anyhow::Error),

    #[error("Database Error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        HttpResponse::build(status_code).json(self.to_string())
    }
}

type Result<T> = std::result::Result<T, ApiError>;

#[get("/ping")]
async fn ping() -> Result<impl Responder> {
    Ok(web::Json("pong"))
}
