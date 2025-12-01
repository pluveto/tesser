# tesser-rpc

`tesser-rpc` lets a Tesser deployment delegate strategy logic to an external process over a strongly typed RPC boundary. The crate bundles the protobuf contracts, a transport-agnostic client trait, and the `RpcStrategy` adapter that plugs into the `tesser-strategy` registry.

## Highlights

- **Complete protobuf schema** – `proto/tesser.proto` defines market data, portfolio snapshots, execution hints, and control-plane requests. `build.rs` uses `tonic-build` so Rust stubs stay in sync.
- **Remote client abstraction** – `RemoteStrategyClient` is the minimal interface the runtime needs. The included `GrpcAdapter` handles channel pooling, retries, request deadlines, and health checks.
- **Drop-in strategy** – `RpcStrategy` implements `tesser_strategy::Strategy`, so setting `kind = "RpcStrategy"` in your TOML config instantly forwards ticks, candles, order books, and fills to the remote service.
- **Liveness guardrails** – Heartbeat loops run in the background and will tear down the transport if the remote endpoint stops responding, preventing stale signals from leaking into the engine.

## Configure the runtime

Add a strategy section to your live configuration:

```toml
[strategies.remote_mm]
kind = "RpcStrategy"

[strategies.remote_mm.params]
transport = "grpc"
endpoint = "http://127.0.0.1:50051"
timeout_ms = 750
symbols = ["BTCUSDT", "ETHUSDT"]
heartbeat_interval_ms = 5000
```

The adapter serializes the entire `params` table and sends it to the remote service during `Initialize`. If the server returns `symbols` in its response, those override the local `subscriptions`.

## Implement a remote strategy service

Use the generated `StrategyService` server stubs to accept callbacks from the runtime:

```rust
use tonic::{Request, Response, Status};
use uuid::Uuid;
use tesser_rpc::proto::strategy_service_server::{StrategyService, StrategyServiceServer};
use tesser_rpc::proto::{signal, Decimal, InitRequest, InitResponse, Signal, SignalList, TickRequest};

#[derive(Default)]
struct ExampleService;

#[tonic::async_trait]
impl StrategyService for ExampleService {
    async fn initialize(&self, request: Request<InitRequest>) -> Result<Response<InitResponse>, Status> {
        let params = request.into_inner().config_json;
        // Inspect params, seed your own state, etc.
        Ok(Response::new(InitResponse {
            success: true,
            error_message: String::new(),
            symbols: vec!["BTCUSDT".into()],
        }))
    }

    async fn on_tick(&self, request: Request<TickRequest>) -> Result<Response<SignalList>, Status> {
        let tick = request.into_inner().tick.expect("tick payload");
        let signals = if tick.symbol == \"BTCUSDT\" {
            vec![Signal {
                symbol: tick.symbol,
                kind: signal::Kind::EnterLong.into(),
                confidence: 0.9,
                stop_loss: None,
                take_profit: None,
                execution_hint: None,
                note: String::new(),
                id: Uuid::new_v4().to_string(),
                generated_at: tick.exchange_timestamp,
                metadata: String::new(),
                quantity: Some(Decimal { value: \"0.5\".into() }),
                group_id: String::new(),
            }]
        } else {
            Vec::new()
        };
        Ok(Response::new(SignalList { signals }))
    }

    // Implement the other RPCs (OnCandle, OnOrderBook, OnFill, Heartbeat) as needed.
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic::transport::Server::builder()
        .add_service(StrategyServiceServer::new(ExampleService::default()))
        .serve("[::1]:50051".parse()?)
        .await?;
    Ok(())
}
```

Generate stubs in your own project by pointing `tonic-build` at `tesser-rpc/proto/tesser.proto`, or depend directly on this crate and reuse the re-exported `proto` module.

## Client transport

`GrpcAdapter` is the default implementation of `RemoteStrategyClient`:

- Configurable per-request deadline via `timeout_ms`.
- Automatic retries for `Unavailable` and `DeadlineExceeded`.
- Background heartbeat loop to surface remote health (set `heartbeat_interval_ms` in config).

You can embed other transports (ZMQ, shared memory, etc.) by implementing `RemoteStrategyClient` and extending the `TransportConfig` enum.

## Testing

- `tests/grpc_e2e.rs` spins up an in-process gRPC server to verify handshake, symbol negotiation, and failover flows.
- Use `StrategyContext` fixtures from `tesser-strategy` and convert them into protobufs with the provided `conversions` module for golden tests.

## Further reading

- `tesser-rpc/src/strategy.rs` – strategy adapter logic, heartbeat management, and signal handling.
- `tesser-rpc/src/transport/grpc.rs` – gRPC transport implementation.
- `docs/content/docs/03-strategy-lab/execution-plugins.mdx` – complementary plugin approach for execution-only WASM modules.
