use crate::init;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::test;
use tokio::sync::OnceCell;

#[allow(unused)]
static INIT: OnceCell<()> = OnceCell::const_new();

#[allow(unused)]
pub async fn test_app() -> impl Service<
    actix_http::Request,
    Response = ServiceResponse<impl actix_http::body::MessageBody>,
    Error = actix_web::Error,
> {
    INIT.get_or_init(|| async {
        init().await;
    })
    .await;

    test::init_service(crate::api::setup_app()).await
}
