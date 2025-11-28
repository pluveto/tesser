use chrono::Utc;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Side of an order emitted by a plugin.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginSide {
    Buy,
    Sell,
}

impl PluginSide {
    /// Convert the side into its multiplier representation (Buy=1, Sell=-1).
    pub fn as_multiplier(self) -> i8 {
        match self {
            Self::Buy => 1,
            Self::Sell => -1,
        }
    }
}

/// Order type supported by plugin order requests.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginOrderType {
    Market,
    Limit,
}

/// Time-in-force policy understood by the orchestrator.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginTimeInForce {
    Gtc,
    Ioc,
    Fok,
    PostOnly,
}

/// Simplified order request structure returned by plugins.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginOrderRequest {
    pub symbol: String,
    pub side: PluginSide,
    pub order_type: PluginOrderType,
    pub quantity: Decimal,
    #[serde(default)]
    pub price: Option<Decimal>,
    #[serde(default)]
    pub trigger_price: Option<Decimal>,
    #[serde(default)]
    pub time_in_force: Option<PluginTimeInForce>,
    #[serde(default)]
    pub client_order_id: Option<String>,
    #[serde(default)]
    pub take_profit: Option<Decimal>,
    #[serde(default)]
    pub stop_loss: Option<Decimal>,
    #[serde(default)]
    pub display_quantity: Option<Decimal>,
}

/// Simplified amendment request emitted by plugins.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginOrderUpdateRequest {
    pub order_id: String,
    pub symbol: String,
    pub side: PluginSide,
    #[serde(default)]
    pub new_price: Option<Decimal>,
    #[serde(default)]
    pub new_quantity: Option<Decimal>,
}

/// Wrapper representing the action a plugin wants the orchestrator to take.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum PluginChildOrderAction {
    Place(PluginOrderRequest),
    Amend(PluginOrderUpdateRequest),
}

/// Response entry returned by plugin callbacks.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginChildOrderRequest {
    pub action: PluginChildOrderAction,
}

/// Data passed to plugins about the originating signal.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginSignal {
    pub id: String,
    pub symbol: String,
    pub side: PluginSide,
    pub kind: String,
    pub confidence: f64,
    pub target_quantity: Decimal,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
}

impl PluginSignal {
    /// Convenience constructor for unit tests/examples.
    pub fn test(symbol: &str, quantity: Decimal) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            symbol: symbol.to_string(),
            side: PluginSide::Buy,
            kind: "enter_long".to_string(),
            confidence: 1.0,
            target_quantity: quantity,
            note: None,
            group_id: None,
        }
    }
}

/// Minimal view of the risk context exposed to the plugin.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginRiskContext {
    pub last_price: Decimal,
    pub portfolio_equity: Decimal,
    pub exchange_equity: Decimal,
    pub signed_position_qty: Decimal,
    pub base_available: Decimal,
    pub quote_available: Decimal,
    pub settlement_available: Decimal,
    #[serde(default)]
    pub instrument_kind: Option<String>,
}

impl Default for PluginRiskContext {
    fn default() -> Self {
        Self {
            last_price: Decimal::ONE,
            portfolio_equity: Decimal::ONE,
            exchange_equity: Decimal::ONE,
            signed_position_qty: Decimal::ZERO,
            base_available: Decimal::ZERO,
            quote_available: Decimal::ZERO,
            settlement_available: Decimal::ZERO,
            instrument_kind: None,
        }
    }
}

/// Context passed to `init` describing the plugin target.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginInitContext {
    pub plugin: String,
    #[serde(default)]
    pub params: Value,
    pub signal: PluginSignal,
    pub risk: PluginRiskContext,
    #[serde(default)]
    pub metadata: Value,
}

/// Representation of a fill routed back into the plugin.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginFill {
    pub order_id: String,
    pub symbol: String,
    pub side: PluginSide,
    pub fill_price: Decimal,
    pub fill_quantity: Decimal,
    #[serde(default)]
    pub fee: Option<Decimal>,
    #[serde(default)]
    pub fee_asset: Option<String>,
    pub timestamp_ms: i64,
}

impl PluginFill {
    pub fn new(
        order_id: impl Into<String>,
        symbol: impl Into<String>,
        side: PluginSide,
        price: Decimal,
        qty: Decimal,
    ) -> Self {
        Self {
            order_id: order_id.into(),
            symbol: symbol.into(),
            side,
            fill_price: price,
            fill_quantity: qty,
            fee: None,
            fee_asset: None,
            timestamp_ms: Utc::now().timestamp_millis(),
        }
    }
}

/// Simplified tick representation forwarded to plugins.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PluginTick {
    pub symbol: String,
    pub price: Decimal,
    pub size: Decimal,
    pub side: PluginSide,
    pub timestamp_ms: i64,
}

impl PluginTick {
    pub fn new(
        symbol: impl Into<String>,
        price: Decimal,
        size: Decimal,
        side: PluginSide,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            symbol: symbol.into(),
            price,
            size,
            side,
            timestamp_ms,
        }
    }
}

/// Canonical plugin callback result.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PluginResult {
    #[serde(default)]
    pub orders: Vec<PluginChildOrderRequest>,
    #[serde(default)]
    pub logs: Vec<String>,
    #[serde(default)]
    pub completed: bool,
}

impl PluginResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_order(mut self, action: PluginChildOrderAction) -> Self {
        self.orders.push(PluginChildOrderRequest { action });
        self
    }

    pub fn completed(mut self) -> Self {
        self.completed = true;
        self
    }
}
