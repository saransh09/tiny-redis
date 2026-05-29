mod command;
mod errors;
mod server;
mod store;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:6379".to_string());
    server::run(&addr).await
}
