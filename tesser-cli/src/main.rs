use std::sync::Arc;

use anyhow::Result;
use tesser_binance::register_factory as register_binance_factory;
use tesser_broker::register_connector_factory;
use tesser_bybit::register_factory as register_bybit_factory;
use tesser_cli::app;
use tesser_paper::PaperFactory;

#[tokio::main]
async fn main() -> Result<()> {
    register_connector_factory(Arc::new(PaperFactory::default()));
    #[cfg(feature = "bybit")]
    register_bybit_factory();
    #[cfg(feature = "binance")]
    register_binance_factory();
    app::run().await
}
