use crate::types::{PluginFill, PluginInitContext, PluginResult, PluginSide, PluginTick};
use once_cell::sync::OnceCell;
use rust_decimal::Decimal;
use serde_json::Value;
use std::sync::Mutex;

#[allow(clippy::too_many_arguments)]
mod bindings {
    wit_bindgen::generate!({
        world: "execution-plugin",
        path: "wit",
    });
}
use bindings::tesser::execution::primitives::Side as AbiSide;
pub use bindings::tesser::execution::primitives::Tick as AbiTick;

/// Trait implemented by plugin authors.
pub trait ExecutionPlugin: Default + 'static {
    fn init(&mut self, ctx: PluginInitContext) -> Result<PluginResult, PluginError>;
    fn on_tick(&mut self, _tick: PluginTick) -> Result<PluginResult, PluginError> {
        Ok(PluginResult::default())
    }
    fn on_fill(&mut self, _fill: PluginFill) -> Result<PluginResult, PluginError> {
        Ok(PluginResult::default())
    }
    fn on_timer(&mut self) -> Result<PluginResult, PluginError> {
        Ok(PluginResult::default())
    }
    fn snapshot(&mut self) -> Result<Value, PluginError> {
        Ok(Value::Null)
    }
    fn restore(&mut self, _state: Value) -> Result<(), PluginError> {
        Ok(())
    }
}

/// Simple error wrapper exposed to plugin authors.
#[derive(Debug)]
pub struct PluginError {
    pub message: String,
}

impl<T> From<T> for PluginError
where
    T: ToString,
{
    fn from(value: T) -> Self {
        Self {
            message: value.to_string(),
        }
    }
}

/// Runtime wrapper that stores a single plugin instance.
pub struct PluginRuntime<P: ExecutionPlugin> {
    inner: OnceCell<Mutex<P>>,
}

impl<P: ExecutionPlugin> Default for PluginRuntime<P> {
    fn default() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }
}

impl<P: ExecutionPlugin> PluginRuntime<P> {
    pub const fn new() -> Self {
        Self {
            inner: OnceCell::new(),
        }
    }

    fn plugin(&self) -> &Mutex<P> {
        self.inner.get_or_init(|| Mutex::new(P::default()))
    }

    fn with_plugin<R>(
        &self,
        f: impl FnOnce(&mut P) -> Result<R, PluginError>,
    ) -> Result<R, PluginError> {
        let mut guard = self
            .plugin()
            .lock()
            .map_err(|_| PluginError::from("plugin lock poisoned"))?;
        f(&mut *guard)
    }

    pub fn call_init(&self, config_json: String) -> Result<String, String> {
        let ctx: PluginInitContext =
            serde_json::from_str(&config_json).map_err(|err| err.to_string())?;
        let response = self
            .with_plugin(|plugin| plugin.init(ctx))
            .map_err(|err| err.message)?;
        serde_json::to_string(&response).map_err(|err| err.to_string())
    }

    pub fn call_on_tick(&self, tick: AbiTick) -> Result<String, String> {
        let tick = convert_tick(tick).map_err(|err| err.message)?;
        let response = self
            .with_plugin(|plugin| plugin.on_tick(tick))
            .map_err(|err| err.message)?;
        serde_json::to_string(&response).map_err(|err| err.to_string())
    }

    pub fn call_on_fill(&self, fill_json: String) -> Result<String, String> {
        let fill: PluginFill = serde_json::from_str(&fill_json).map_err(|err| err.to_string())?;
        let response = self
            .with_plugin(|plugin| plugin.on_fill(fill))
            .map_err(|err| err.message)?;
        serde_json::to_string(&response).map_err(|err| err.to_string())
    }

    pub fn call_on_timer(&self) -> Result<String, String> {
        let response = self
            .with_plugin(|plugin| plugin.on_timer())
            .map_err(|err| err.message)?;
        serde_json::to_string(&response).map_err(|err| err.to_string())
    }

    pub fn call_snapshot(&self) -> Result<String, String> {
        let snapshot = self
            .with_plugin(|plugin| plugin.snapshot())
            .map_err(|err| err.message)?;
        serde_json::to_string(&snapshot).map_err(|err| err.to_string())
    }

    pub fn call_restore(&self, state_json: String) -> Result<(), String> {
        let state: Value = serde_json::from_str(&state_json).map_err(|err| err.to_string())?;
        self.with_plugin(|plugin| plugin.restore(state))
            .map_err(|err| err.message)
    }
}

fn convert_tick(source: AbiTick) -> Result<PluginTick, PluginError> {
    let price = Decimal::from_str_exact(&source.price.value)
        .map_err(|err| PluginError::from(format!("invalid price: {err}")))?;
    let size = Decimal::from_str_exact(&source.size.value)
        .map_err(|err| PluginError::from(format!("invalid size: {err}")))?;
    Ok(PluginTick {
        symbol: source.symbol,
        price,
        size,
        side: match source.side {
            AbiSide::Buy => PluginSide::Buy,
            AbiSide::Sell => PluginSide::Sell,
        },
        timestamp_ms: source.timestamp_ms,
    })
}

#[macro_export]
macro_rules! export_plugin {
    ($ty:ty) => {
        struct __TesserWasmPlugin;
        static RUNTIME: $crate::guest::PluginRuntime<$ty> = $crate::guest::PluginRuntime::new();

        impl $crate::guest::WasmGuest for __TesserWasmPlugin {
            fn init(config_json: String) -> Result<String, String> {
                RUNTIME.call_init(config_json)
            }

            fn on_tick(tick: $crate::guest::AbiTick) -> Result<String, String> {
                RUNTIME.call_on_tick(tick)
            }

            fn on_fill(fill_json: String) -> Result<String, String> {
                RUNTIME.call_on_fill(fill_json)
            }

            fn on_timer() -> Result<String, String> {
                RUNTIME.call_on_timer()
            }

            fn snapshot() -> Result<String, String> {
                RUNTIME.call_snapshot()
            }

            fn restore(state_json: String) -> Result<(), String> {
                RUNTIME.call_restore(state_json)
            }
        }

        wit_bindgen::export!(__TesserWasmPlugin);
    };
}
