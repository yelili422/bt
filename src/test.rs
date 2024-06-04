use actix_web::dev::{Service, ServiceResponse};
use actix_web::test;

use crate::downloader::{update_torrent_cache, Torrent, TorrentMeta};
use crate::init;

#[allow(unused)]
pub async fn test_app() -> impl Service<
    actix_http::Request,
    Response = ServiceResponse<impl actix_http::body::MessageBody>,
    Error = actix_web::Error,
> {
    init().await;

    test::init_service(crate::api::setup_app()).await
}

#[allow(unused)]
pub fn gen_torrent_with_custom_filename(filename: &str) -> Torrent {
    let torrent_content = format!(
        "d8:announce0:4:infod6:lengthi0e4:name{}:{}12:piece lengthi0e6:pieces0:ee",
        filename.len(),
        filename
    );
    Torrent::from_bytes(torrent_content.as_bytes()).unwrap()
}

#[allow(unused)]
pub async fn get_dummy_torrent() -> TorrentMeta {
    let dot_torrent =
        std::fs::read("tests/dataset/872ab5abd72ea223d2a2e36688cc96f83bb71d42.torrent").unwrap();
    let torrent = Torrent::from_bytes(&dot_torrent).unwrap();

    let url = "https://example.com/dummy-1.torrent";
    update_torrent_cache(url, &torrent).await;

    TorrentMeta::builder()
        .url(url.to_string())
        .category(Some("test_category".to_string()))
        .build()
}
