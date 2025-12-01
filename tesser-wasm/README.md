# tesser-wasm

`tesser-wasm` is the WebAssembly SDK that lets you author custom execution plugins for the Tesser runtime. It defines the ABI shared between the Rust orchestrator and your plugin, exposes ergonomic data types (`PluginSignal`, `PluginRiskContext`, `PluginResult`, …), and provides a small helper runtime so you can focus exclusively on your execution logic.

## Feature flags

| Feature | When to use it | What you get |
| --- | --- | --- |
| _default_ | Any crate that only needs the shared types (host-side orchestration, tests, docs) | All serialization types and helpers. |
| `guest` | Crates that compile into WASI modules (plugins) | `ExecutionPlugin` trait, `PluginRuntime`, `export_plugin!` macro, and the generated bindings from `wit-bindgen`. |

## Plugin quick start

1. **Scaffold a crate**

   ```bash
   cargo new chase-execution --lib
   cd chase-execution
   ```

2. **Update `Cargo.toml`**

   ```toml
   [lib]
   crate-type = ["cdylib"]

   [dependencies]
   rust_decimal = "1"
   tesser-wasm = { path = "../../tesser-wasm", features = ["guest"] }
   ```

3. **Implement the trait and export it**

   ```rust
   use rust_decimal::Decimal;
   use tesser_wasm::{
       export_plugin, ExecutionPlugin, PluginChildOrderAction, PluginInitContext,
       PluginOrderRequest, PluginOrderType, PluginResult, PluginSide, PluginTick,
   };

   #[derive(Default)]
   struct ChasePlugin {
       symbol: String,
       side: PluginSide,
       remaining: Decimal,
       last_price: Decimal,
   }

   impl ExecutionPlugin for ChasePlugin {
       fn init(&mut self, ctx: PluginInitContext) -> Result<PluginResult, tesser_wasm::PluginError> {
           self.symbol = ctx.signal.symbol;
           self.side = ctx.signal.side;
           self.remaining = ctx.signal.target_quantity;
           self.last_price = ctx.risk.last_price;
           Ok(PluginResult::new())
       }

       fn on_tick(&mut self, tick: PluginTick) -> Result<PluginResult, tesser_wasm::PluginError> {
           self.last_price = tick.price;
           Ok(PluginResult::new())
       }

       fn on_timer(&mut self) -> Result<PluginResult, tesser_wasm::PluginError> {
           if self.remaining <= Decimal::ZERO {
               return Ok(PluginResult::new().completed());
           }
           let slice = self.remaining.min(Decimal::new(1, 1));
           self.remaining -= slice;
           let request = PluginOrderRequest {
               symbol: self.symbol.clone(),
               side: self.side,
               order_type: PluginOrderType::Limit,
               quantity: slice,
               price: Some(self.last_price),
               trigger_price: None,
               time_in_force: None,
               client_order_id: None,
               take_profit: None,
               stop_loss: None,
               display_quantity: None,
           };
           Ok(PluginResult::new().with_order(PluginChildOrderAction::Place(request)))
       }
   }

   export_plugin!(ChasePlugin);
   ```

4. **Compile for WASI**

   ```bash
   rustup target add wasm32-wasi # once per workstation
   cargo build --release --target wasm32-wasi
   ```

   The artifact is emitted at `target/wasm32-wasi/release/<crate_name>.wasm`.

## Loading the module in Tesser

1. Configure the orchestrator to search for plugins:

   ```toml
   [live]
   plugins_dir = "./plugins"
   ```

   or pass `--plugins-dir` to `tesser live run`.

2. Drop your compiled `<crate_name>.wasm` into that directory.

3. Attach an execution hint when emitting a signal:

   ```rust
   use serde_json::json;
   use tesser_core::{ExecutionHint, Signal, SignalKind};

   let signal = Signal::new("BTCUSDT", SignalKind::EnterLong, 0.8).with_hint(
       ExecutionHint::Plugin {
           name: "chase_execution".into(),
           params: json!({ "clip_size": "0.25" }),
       },
   );
   ctx.publish(signal);
   ```

The runtime instantiates your module, calls `init`, then forwards ticks, fills, and timer heartbeats into the plugin. You can persist lightweight JSON snapshots via `snapshot` / `restore`, emit structured logs through `PluginResult.logs`, and return child order actions to delegate to the core orchestration engine. `examples/plugin-chase` contains a fully working reference implementation.

## Host-side usage

Crates such as `tesser-execution` depend on `tesser-wasm` (without the `guest` feature) to deserialize plugin responses, mock plugins in tests, and manage persistence. The `PluginRuntime<P>` helper included in the `guest` feature can also be reused in host-side integration tests to drive an actual plugin using JSON fixtures.

## Additional resources

- [Execution plugins guide](https://www.tesser.space/docs/03-strategy-lab/execution-plugins)
- `examples/plugin-chase` – minimal end-to-end chase algorithm
- `tesser-execution/src/wasm/engine.rs` – host-side loader implementation

The crate inherits the workspace licensing terms (Apache-2.0 OR MIT). Contributions are welcome—follow the repository guidelines before opening a PR.
