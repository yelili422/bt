use crate::init;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::test;

#[allow(unused)]
pub async fn test_app() -> impl Service<
    actix_http::Request,
    Response = ServiceResponse<impl actix_http::body::MessageBody>,
    Error = actix_web::Error,
> {
    init().await;

    test::init_service(crate::api::setup_app()).await
}
