use async_trait::async_trait;
use serde::Deserialize;
use tesser_core::{Candle, Fill, OrderBook, Signal, Symbol, Tick};
use tesser_strategy::{
    register_strategy, Strategy, StrategyContext, StrategyError, StrategyResult,
};
use tracing::{error, info};

use crate::client::RemoteStrategyClient;
use crate::proto::{CandleRequest, FillRequest, InitRequest, OrderBookRequest, TickRequest};
use crate::transport::grpc::GrpcAdapter;

#[derive(Clone, Deserialize)]
#[serde(tag = "transport")]
enum TransportConfig {
    #[serde(rename = "grpc")]
    Grpc {
        endpoint: String,
        #[serde(default = "default_timeout_ms")]
        timeout_ms: u64,
    },
    // Future expansion: ZMQ, SHM, etc.
}

fn default_timeout_ms() -> u64 {
    500
}

/// A strategy adapter that delegates decision making to an external service via a pluggable transport.
pub struct RpcStrategy {
    client: Option<Box<dyn RemoteStrategyClient>>,
    transport_config: Option<TransportConfig>,
    config_payload: String,
    subscriptions: Vec<String>,
    pending_signals: Vec<Signal>,
    symbol: String, // Primary symbol fallback
}

impl Default for RpcStrategy {
    fn default() -> Self {
        Self {
            client: None,
            transport_config: None,
            config_payload: "{}".to_string(),
            subscriptions: vec![],
            pending_signals: vec![],
            symbol: "UNKNOWN".to_string(),
        }
    }
}

impl RpcStrategy {
    fn build_client(config: &TransportConfig) -> Box<dyn RemoteStrategyClient> {
        match config {
            TransportConfig::Grpc {
                endpoint,
                timeout_ms,
            } => {
                info!(target: "rpc", endpoint, "configured gRPC transport");
                Box::new(GrpcAdapter::new(endpoint.clone(), *timeout_ms))
            }
        }
    }

    async fn ensure_client(&mut self) -> StrategyResult<&mut (dyn RemoteStrategyClient + '_)> {
        if self.client.is_none() {
            let config = self
                .transport_config
                .clone()
                .ok_or_else(|| StrategyError::InvalidConfig("transport config missing".into()))?;

            let mut client = Self::build_client(&config);

            client
                .connect()
                .await
                .map_err(|e| StrategyError::Internal(format!("RPC connect failed: {e}")))?;

            let init_request = InitRequest {
                config_json: self.config_payload.clone(),
            };

            let response = client.initialize(init_request).await.map_err(|e| {
                StrategyError::Internal(format!("remote strategy init failed: {e}"))
            })?;

            if !response.success {
                return Err(StrategyError::Internal(format!(
                    "remote strategy rejected init: {}",
                    response.error_message
                )));
            }

            self.apply_remote_metadata(response.symbols);
            info!(target: "rpc", symbols = ?self.subscriptions, "RPC strategy initialized");
            self.client = Some(client);
        }

        match self.client.as_deref_mut() {
            Some(client) => Ok(client),
            None => Err(StrategyError::Internal("RPC client not initialized".into())),
        }
    }

    fn apply_remote_metadata(&mut self, mut symbols: Vec<String>) {
        if symbols.is_empty() {
            symbols.push(self.symbol.clone());
        }
        if let Some(primary) = symbols.first() {
            self.symbol = primary.clone();
        }
        self.subscriptions = symbols;
    }

    fn handle_signals(&mut self, signals: Vec<crate::proto::Signal>) {
        for proto_sig in signals {
            self.pending_signals.push(proto_sig.into());
        }
    }
}

#[async_trait]
impl Strategy for RpcStrategy {
    fn name(&self) -> &str {
        "rpc-strategy"
    }

    fn symbol(&self) -> &str {
        &self.symbol
    }

    fn subscriptions(&self) -> Vec<Symbol> {
        if self.subscriptions.is_empty() {
            vec![self.symbol.clone()]
        } else {
            self.subscriptions.clone()
        }
    }

    fn configure(&mut self, params: toml::Value) -> StrategyResult<()> {
        let config: TransportConfig = params.clone().try_into().map_err(|e| {
            StrategyError::InvalidConfig(format!("failed to parse RPC config: {}", e))
        })?;

        self.transport_config = Some(config);
        self.client = None;
        self.subscriptions.clear();
        self.symbol = "UNKNOWN".to_string();
        self.pending_signals.clear();
        self.config_payload = serde_json::to_string(&params).unwrap_or_else(|_| "{}".to_string());
        Ok(())
    }

    async fn on_tick(&mut self, ctx: &StrategyContext, tick: &Tick) -> StrategyResult<()> {
        let request = TickRequest {
            tick: Some(tick.clone().into()),
            context: Some(ctx.into()),
        };

        let client = self.ensure_client().await?;
        match client.on_tick(request).await {
            Ok(response) => self.handle_signals(response.signals),
            Err(e) => error!("RPC OnTick error: {}", e),
        }
        Ok(())
    }

    async fn on_candle(&mut self, ctx: &StrategyContext, candle: &Candle) -> StrategyResult<()> {
        let request = CandleRequest {
            candle: Some(candle.clone().into()),
            context: Some(ctx.into()),
        };

        let client = self.ensure_client().await?;
        match client.on_candle(request).await {
            Ok(response) => self.handle_signals(response.signals),
            Err(e) => error!("RPC OnCandle error: {}", e),
        }
        Ok(())
    }

    async fn on_fill(&mut self, ctx: &StrategyContext, fill: &Fill) -> StrategyResult<()> {
        let request = FillRequest {
            fill: Some(fill.clone().into()),
            context: Some(ctx.into()),
        };

        let client = self.ensure_client().await?;
        match client.on_fill(request).await {
            Ok(response) => self.handle_signals(response.signals),
            Err(e) => error!("RPC OnFill error: {}", e),
        }
        Ok(())
    }

    async fn on_order_book(
        &mut self,
        ctx: &StrategyContext,
        book: &OrderBook,
    ) -> StrategyResult<()> {
        let request = OrderBookRequest {
            order_book: Some(book.clone().into()),
            context: Some(ctx.into()),
        };

        let client = self.ensure_client().await?;
        match client.on_order_book(request).await {
            Ok(response) => self.handle_signals(response.signals),
            Err(e) => error!("RPC OnOrderBook error: {}", e),
        }
        Ok(())
    }

    fn drain_signals(&mut self) -> Vec<Signal> {
        std::mem::take(&mut self.pending_signals)
    }
}

register_strategy!(RpcStrategy, "RpcStrategy");
