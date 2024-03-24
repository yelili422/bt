use actix_web::{delete, get, post, put, Responder, web};
use crate::{rss, tx_begin};
use crate::rss::store::RssEntity;

#[get("")]
async fn get_rss() -> crate::api::Result<impl Responder> {
    let mut tx = tx_begin().await?;
    let rss_list = rss::store::get_rss_list(&mut tx).await?;
    tx.rollback().await?;
    Ok(web::Json(rss_list))
}

#[post("")]
async fn add_rss(info: web::Json<RssEntity>) -> crate::api::Result<impl Responder> {
    let mut tx = tx_begin().await?;
    let id = rss::store::add_rss(&mut tx, &info.into_inner()).await?;
    tx.commit().await?;
    Ok(web::Json(id))
}

#[delete("/{id}")]
async fn delete_rss(path: web::Path<i64>) -> crate::api::Result<impl Responder> {
    let mut tx = tx_begin().await?;
    rss::store::delete_rss(&mut tx, path.into_inner()).await?;
    tx.commit().await?;
    Ok(web::Json("ok"))
}

#[put("/{id}")]
async fn update_rss(path: web::Path<i64>, info: web::Json<RssEntity>) -> crate::api::Result<impl Responder> {
    let mut tx = tx_begin().await?;
    rss::store::update_rss(&mut tx, path.into_inner(), &info.into_inner()).await?;
    tx.commit().await?;
    Ok(web::Json("ok"))
}
