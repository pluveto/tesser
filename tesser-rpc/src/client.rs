use crate::proto::{
    CandleRequest, FillRequest, HeartbeatResponse, InitRequest, InitResponse, OrderBookRequest,
    SignalList, TickRequest,
};
use anyhow::Result;
use async_trait::async_trait;

/// Transport-agnostic interface for communicating with external strategies.
#[async_trait]
pub trait RemoteStrategyClient: Send + Sync {
    /// Establishes the connection to the remote strategy.
    async fn connect(&mut self) -> Result<()>;

    /// Performs the initial handshake and configuration.
    async fn initialize(&mut self, req: InitRequest) -> Result<InitResponse>;

    /// Pushes a tick event.
    async fn on_tick(&mut self, req: TickRequest) -> Result<SignalList>;

    /// Pushes a candle event.
    async fn on_candle(&mut self, req: CandleRequest) -> Result<SignalList>;

    /// Pushes an order book snapshot.
    async fn on_order_book(&mut self, req: OrderBookRequest) -> Result<SignalList>;

    /// Pushes an execution fill.
    async fn on_fill(&mut self, req: FillRequest) -> Result<SignalList>;

    /// Heartbeat to verify the remote strategy is still reachable.
    async fn heartbeat(&mut self) -> Result<HeartbeatResponse>;
}
