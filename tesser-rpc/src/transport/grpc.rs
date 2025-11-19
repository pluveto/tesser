use anyhow::{anyhow, Result};
use async_trait::async_trait;
use prost_types::Timestamp;
use std::future::Future;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tonic::transport::{Channel, Endpoint};
use tonic::{Code, Status};
use tracing::{debug, warn};

use crate::client::RemoteStrategyClient;
use crate::proto::strategy_service_client::StrategyServiceClient;
use crate::proto::{
    CandleRequest, FillRequest, HeartbeatRequest, HeartbeatResponse, InitRequest, InitResponse,
    OrderBookRequest, SignalList, TickRequest,
};

/// A gRPC-based implementation of the strategy client.
pub struct GrpcAdapter {
    endpoint: String,
    client: Option<StrategyServiceClient<Channel>>,
    timeout: Duration,
    max_retries: u32,
}

impl GrpcAdapter {
    pub fn new(endpoint: String, timeout_ms: u64) -> Self {
        Self {
            endpoint,
            client: None,
            timeout: Duration::from_millis(timeout_ms.max(1)),
            max_retries: 3,
        }
    }

    fn should_retry(&self, attempts: u32, status: &Status) -> bool {
        attempts < self.max_retries
            && matches!(status.code(), Code::Unavailable | Code::DeadlineExceeded)
    }

    async fn call_with_retry<T, F, Fut>(&mut self, mut op: F) -> Result<T>
    where
        F: FnMut(StrategyServiceClient<Channel>) -> Fut,
        Fut: Future<Output = (StrategyServiceClient<Channel>, Result<T, Status>)>,
    {
        let mut attempts = 0;
        loop {
            if self.client.is_none() {
                self.connect().await?;
            }
            attempts += 1;
            let client = self
                .client
                .take()
                .ok_or_else(|| anyhow!("gRPC client missing"))?;
            let (client, result) = op(client).await;
            match result {
                Ok(value) => {
                    self.client = Some(client);
                    return Ok(value);
                }
                Err(status) if self.should_retry(attempts, &status) => {
                    warn!(
                        target: "rpc",
                        attempt = attempts,
                        code = ?status.code(),
                        "gRPC call failed; retrying"
                    );
                    self.client = None;
                }
                Err(status) => {
                    self.client = Some(client);
                    return Err(anyhow!(status));
                }
            }
        }
    }

    fn heartbeat_request() -> HeartbeatRequest {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        HeartbeatRequest {
            timestamp: Some(Timestamp {
                seconds: now.as_secs() as i64,
                nanos: now.subsec_nanos() as i32,
            }),
        }
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
        let payload = req;
        self.call_with_retry(move |mut client| {
            let mut request = tonic::Request::new(payload.clone());
            request.set_timeout(timeout);
            async move {
                let response = client
                    .initialize(request)
                    .await
                    .map(|resp| resp.into_inner());
                (client, response)
            }
        })
        .await
    }

    async fn on_tick(&mut self, req: TickRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let payload = req;
        self.call_with_retry(move |mut client| {
            let mut request = tonic::Request::new(payload.clone());
            request.set_timeout(timeout);
            async move {
                let response = client.on_tick(request).await.map(|resp| resp.into_inner());
                (client, response)
            }
        })
        .await
    }

    async fn on_candle(&mut self, req: CandleRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let payload = req;
        self.call_with_retry(move |mut client| {
            let mut request = tonic::Request::new(payload.clone());
            request.set_timeout(timeout);
            async move {
                let response = client
                    .on_candle(request)
                    .await
                    .map(|resp| resp.into_inner());
                (client, response)
            }
        })
        .await
    }

    async fn on_order_book(&mut self, req: OrderBookRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let payload = req;
        self.call_with_retry(move |mut client| {
            let mut request = tonic::Request::new(payload.clone());
            request.set_timeout(timeout);
            async move {
                let response = client
                    .on_order_book(request)
                    .await
                    .map(|resp| resp.into_inner());
                (client, response)
            }
        })
        .await
    }

    async fn on_fill(&mut self, req: FillRequest) -> Result<SignalList> {
        let timeout = self.timeout;
        let payload = req;
        self.call_with_retry(move |mut client| {
            let mut request = tonic::Request::new(payload.clone());
            request.set_timeout(timeout);
            async move {
                let response = client.on_fill(request).await.map(|resp| resp.into_inner());
                (client, response)
            }
        })
        .await
    }

    async fn heartbeat(&mut self) -> Result<HeartbeatResponse> {
        let timeout = self.timeout;
        self.call_with_retry(move |mut client| {
            let mut request = tonic::Request::new(Self::heartbeat_request());
            request.set_timeout(timeout);
            async move {
                let response = client
                    .heartbeat(request)
                    .await
                    .map(|resp| resp.into_inner());
                (client, response)
            }
        })
        .await
    }
}
