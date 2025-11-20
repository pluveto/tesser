use std::sync::Arc;

use anyhow::Result;
use tesser_broker::register_connector_factory;
use tesser_cli::app;
use tesser_paper::PaperFactory;

#[tokio::main]
async fn main() -> Result<()> {
    register_connector_factory(Arc::new(PaperFactory::default()));
    app::run().await
}
