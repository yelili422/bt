mod api;

use bt::init;

#[tokio::main]
async fn main() {
    init().await;
    api::run().await.unwrap();
}
