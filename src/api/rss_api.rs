use actix_web::{delete, get, post, put, web, Responder};

use crate::rss;

#[get("")]
async fn get_rss() -> crate::api::Result<impl Responder> {
    let rss_list = rss::list_rss().await?;
    Ok(web::Json(rss_list))
}

#[post("")]
async fn add_rss(info: web::Json<rss::Rss>) -> crate::api::Result<impl Responder> {
    let id = rss::add_rss(&info.into_inner()).await?;
    Ok(web::Json(id))
}

#[delete("/{id}")]
async fn delete_rss(path: web::Path<i64>) -> crate::api::Result<impl Responder> {
    rss::delete_rss(path.into_inner()).await?;
    Ok(web::Json("ok"))
}

#[put("/{id}")]
async fn update_rss(
    path: web::Path<i64>,
    info: web::Json<rss::Rss>,
) -> crate::api::Result<impl Responder> {
    rss::update_rss(path.into_inner(), &info.into_inner()).await?;
    Ok(web::Json("ok"))
}
