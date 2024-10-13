use actix_web::{web, HttpRequest, HttpResponse, Responder};
use bt::{renamer, rss, BTError};
use serde_json::json;

use super::ApiResult;

pub async fn get_rss() -> ApiResult<impl Responder> {
    let rss_list = rss::store::query_rss().await.map_err(BTError::from)?;
    Ok(web::Json(rss_list))
}

pub async fn add_rss(info: web::Json<rss::Rss>) -> ApiResult<impl Responder> {
    let id = rss::store::add_rss(&info.into_inner())
        .await
        .map_err(BTError::from)?;
    Ok(web::Json(id))
}

pub async fn delete_rss(path: web::Path<i64>) -> ApiResult<impl Responder> {
    rss::store::delete_rss(path.into_inner())
        .await
        .map_err(BTError::from)?;
    Ok(web::Json("ok"))
}

pub async fn update_rss(
    path: web::Path<i64>,
    info: web::Json<rss::Rss>,
) -> ApiResult<impl Responder> {
    rss::store::update_rss(path.into_inner(), &info.into_inner())
        .await
        .map_err(BTError::from)?;
    Ok(web::Json("ok"))
}

pub async fn parse_rss(req: HttpRequest) -> ApiResult<impl Responder> {
    let info = web::Query::<rss::Rss>::from_query(req.query_string());
    if info.is_err() {
        return Ok(HttpResponse::BadRequest().json("Invalid RSS"));
    }

    let info = info.unwrap();
    let feeds = rss::parsers::parse(&info).await.map_err(BTError::from)?;
    let paths: Vec<_> = feeds
        .items
        .iter()
        .map(|feed| renamer::BangumiInfo::from(feed).gen_path("(mkv|mp4)"))
        .collect();
    Ok(HttpResponse::Ok().json(json!({
        "rss": feeds,
        "paths": paths,
    })))
}
