#[tokio::main]
async fn main() -> anyhow::Result<()> {
    arrgh_server::run().await
}
