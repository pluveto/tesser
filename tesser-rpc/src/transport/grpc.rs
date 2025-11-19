use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::time::Duration;
use tonic::transport::{Channel, Endpoint};
use tracing::debug;

use crate::client::RemoteStrategyClient;
use crate::proto::strategy_service_client::StrategyServiceClient;
use crate::proto::{
    CandleRequest, FillRequest, InitRequest, InitResponse, OrderBookRequest, SignalList,
    TickRequest,
};

/// A gRPC-based implementation of the strategy client.
pub struct GrpcAdapter {
    endpoint: String,
    client: Option<StrategyServiceClient<Channel>>,
    timeout: Duration,
}

impl GrpcAdapter {
    pub fn new(endpoint: String, timeout_ms: u64) -> Self {
        Self {
            endpoint,
            client: None,
            timeout: Duration::from_millis(timeout_ms.max(1)),
        }
    }

    fn client_mut(&mut self) -> Result<&mut StrategyServiceClient<Channel>> {
        self.client
            .as_mut()
            .ok_or_else(|| anyhow!("gRPC client not connected"))
    }
}

#[async_trait]
impl RemoteStrategyClient for GrpcAdapter {
    async fn connect(&mut self) -> Result<()> {
        debug!("connecting to gRPC strategy at {}", self.endpoint);
        let channel = Endpoint::from_shared(self.endpoint.clone())?
            .connect_timeout(self.timeout)
            .timeout(self.timeout)
            .connect()
            .await?;
        self.client = Some(StrategyServiceClient::new(channel));
        Ok(())
    }

    async fn initialize(&mut self, req: InitRequest) -> Result<InitResponse> {
        let timeout = self.timeout;
        let client = self.client_mut()?;
        let mut request = tonic::Request::new(req);
        request.set_timeout(timeout);
        let response = client.initialize(request).await?;
        Ok(response.into_inner())
    }

    async fn on_tick(&mut self, req: TickRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let client = self.client_mut()?;
        let mut request = tonic::Request::new(req);
        request.set_timeout(timeout);
        let response = client.on_tick(request).await?;
        Ok(response.into_inner())
    }

    async fn on_candle(&mut self, req: CandleRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let client = self.client_mut()?;
        let mut request = tonic::Request::new(req);
        request.set_timeout(timeout);
        let response = client.on_candle(request).await?;
        Ok(response.into_inner())
    }

    async fn on_order_book(&mut self, req: OrderBookRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let client = self.client_mut()?;
        let mut request = tonic::Request::new(req);
        request.set_timeout(timeout);
        let response = client.on_order_book(request).await?;
        Ok(response.into_inner())
    }

    async fn on_fill(&mut self, req: FillRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let client = self.client_mut()?;
        let mut request = tonic::Request::new(req);
        request.set_timeout(timeout);
        let response = client.on_fill(request).await?;
        Ok(response.into_inner())
    }
}
