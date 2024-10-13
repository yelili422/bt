mod rss_api;

use actix_http::body::MessageBody;
use actix_web::dev::{ServiceFactory, ServiceRequest, ServiceResponse};
use actix_web::http::StatusCode;
use actix_web::middleware::Logger;
use actix_web::{get, web, App, Error, HttpResponse, HttpServer, Responder, ResponseError};
use bt::BTError;
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
        .service(web::resource("/preview").route(web::get().to(parse_rss)))
        .service(
            web::resource("")
                .route(web::get().to(get_rss))
                .route(web::post().to(add_rss)),
        )
        .service(
            web::resource("/{id}")
                .route(web::delete().to(delete_rss))
                .route(web::put().to(update_rss)),
        );

    App::new()
        .wrap(Logger::default())
        .service(ping)
        .service(rss_scope)
}

#[get("/ping")]
async fn ping() -> ApiResult<impl Responder> {
    Ok(web::Json("pong"))
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("Internal Error: {0}")]
    InternalError(#[from] BTError),
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        HttpResponse::build(status_code).json(self.to_string())
    }
}

type ApiResult<T> = Result<T, ApiError>;
