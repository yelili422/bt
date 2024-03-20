use bt::api;
use dotenvy::dotenv;

#[tokio::main]
async fn main() {
    _ = dotenv();
    env_logger::init();
    api::run().await.unwrap();
}
