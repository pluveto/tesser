use rust_decimal::prelude::FromStr;
use rust_decimal::Decimal;
use tesser_wasm::{
    export_plugin, ExecutionPlugin, PluginChildOrderAction, PluginInitContext, PluginOrderRequest,
    PluginOrderType, PluginResult, PluginSide, PluginTick,
};

#[derive(Default)]
struct ChasePlugin {
    symbol: String,
    side: PluginSide,
    remaining: Decimal,
    clip_size: Decimal,
    last_price: Decimal,
}

impl ExecutionPlugin for ChasePlugin {
    fn init(&mut self, ctx: PluginInitContext) -> Result<PluginResult, tesser_wasm::PluginError> {
        self.symbol = ctx.signal.symbol;
        self.side = ctx.signal.side;
        self.remaining = ctx.signal.target_quantity.max(Decimal::ZERO);
        self.clip_size = ctx
            .params
            .get("clip_size")
            .and_then(|value| value.as_str())
            .and_then(|raw| Decimal::from_str_exact(raw).ok())
            .unwrap_or_else(|| Decimal::new(1, 0));
        self.last_price = ctx.risk.last_price.max(Decimal::ONE);
        Ok(PluginResult::default())
    }

    fn on_tick(&mut self, tick: PluginTick) -> Result<PluginResult, tesser_wasm::PluginError> {
        self.last_price = tick.price;
        Ok(PluginResult::default())
    }

    fn on_timer(&mut self) -> Result<PluginResult, tesser_wasm::PluginError> {
        if self.remaining <= Decimal::ZERO {
            return Ok(PluginResult::default());
        }
        let slice = self.clip_size.min(self.remaining);
        self.remaining -= slice;
        let price = self.last_price;
        let order = PluginOrderRequest {
            symbol: self.symbol.clone(),
            side: self.side,
            order_type: PluginOrderType::Limit,
            quantity: slice,
            price: Some(price),
            trigger_price: None,
            time_in_force: None,
            client_order_id: None,
            take_profit: None,
            stop_loss: None,
            display_quantity: None,
        };
        Ok(PluginResult::default().with_order(PluginChildOrderAction::Place(order)))
    }
}

export_plugin!(ChasePlugin);
