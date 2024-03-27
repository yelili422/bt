use bt::{api, init};

#[tokio::main]
async fn main() {
    init().await;
    api::run().await.unwrap();
}
