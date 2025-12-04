//! Host-side bindings shared with the execution runtime.

mod bindings {
    wasmtime::component::bindgen!({
        world: "execution-plugin",
        path: "wit",
    });
}

pub use bindings::tesser::execution::primitives::{
    DecimalValue, Side as WasiSide, Tick as WasiTick,
};
pub use bindings::ExecutionPlugin as ComponentBindings;
