use tokio::signal;


#[tokio::main]
async fn main() {
    tokio::spawn(async {
        println!("Hello, world!");
        
    });

    tokio::select! {
        _ = signal::ctrl_c() => {
            println!("shutting down...")
        }
    }
}
