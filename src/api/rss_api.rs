use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde_json::json;

use crate::{renamer, rss, BTResult};

pub async fn get_rss() -> BTResult<impl Responder> {
    let rss_list = rss::list_rss().await?;
    Ok(web::Json(rss_list))
}

pub async fn add_rss(info: web::Json<rss::Rss>) -> BTResult<impl Responder> {
    let id = rss::add_rss(&info.into_inner()).await?;
    Ok(web::Json(id))
}

pub async fn delete_rss(path: web::Path<i64>) -> BTResult<impl Responder> {
    rss::delete_rss(path.into_inner()).await?;
    Ok(web::Json("ok"))
}

pub async fn update_rss(
    path: web::Path<i64>,
    info: web::Json<rss::Rss>,
) -> BTResult<impl Responder> {
    rss::update_rss(path.into_inner(), &info.into_inner()).await?;
    Ok(web::Json("ok"))
}

pub async fn parse_rss(req: HttpRequest) -> BTResult<impl Responder> {
    let info = web::Query::<rss::Rss>::from_query(req.query_string());
    if info.is_err() {
        return Ok(HttpResponse::BadRequest().json("Invalid RSS"));
    }

    let info = info.unwrap();
    let feeds = rss::parsers::parse(&info).await?;
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
