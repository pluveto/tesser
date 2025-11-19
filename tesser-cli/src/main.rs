use anyhow::Result;
use tesser_cli::app;

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
