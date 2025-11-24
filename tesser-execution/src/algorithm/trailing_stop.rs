use anyhow::{anyhow, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AlgoStatus, ChildOrderAction, ChildOrderRequest, ExecutionAlgorithm};
use tesser_core::{Fill, Order, OrderRequest, OrderType, Price, Quantity, Side, Signal, Tick};

#[derive(Debug, Deserialize, Serialize)]
struct TrailingStopState {
    id: Uuid,
    parent_signal: Signal,
    status: String,
    total_quantity: Quantity,
    filled_quantity: Quantity,
    activation_price: Price,
    callback_rate: Decimal,
    highest_market_price: Price,
    activated: bool,
    triggered: bool,
}

/// Simple trailing stop that arms once price trades through an activation level and
/// fires a market sell when price retraces by the configured callback percentage.
pub struct TrailingStopAlgorithm {
    state: TrailingStopState,
}

impl TrailingStopAlgorithm {
    pub fn new(
        signal: Signal,
        total_quantity: Quantity,
        activation_price: Price,
        callback_rate: Decimal,
    ) -> Result<Self> {
        if total_quantity <= Decimal::ZERO {
            return Err(anyhow!("trailing stop quantity must be positive"));
        }
        if signal.kind.side() != Side::Sell {
            return Err(anyhow!(
                "trailing stop currently supports sell-side signals only"
            ));
        }
        if activation_price <= Decimal::ZERO {
            return Err(anyhow!("activation price must be positive"));
        }
        if callback_rate <= Decimal::ZERO || callback_rate >= Decimal::ONE {
            return Err(anyhow!("callback rate must be between 0 and 1"));
        }

        Ok(Self {
            state: TrailingStopState {
                id: Uuid::new_v4(),
                parent_signal: signal,
                status: "Working".into(),
                total_quantity,
                filled_quantity: Decimal::ZERO,
                activation_price,
                callback_rate,
                highest_market_price: activation_price,
                activated: false,
                triggered: false,
            },
        })
    }

    fn remaining(&self) -> Quantity {
        (self.state.total_quantity - self.state.filled_quantity).max(Decimal::ZERO)
    }

    fn try_activate(&mut self, price: Price) {
        if !self.state.activated && price >= self.state.activation_price {
            self.state.activated = true;
            self.state.highest_market_price = price;
        }
    }

    fn update_trail(&mut self, price: Price) {
        if price > self.state.highest_market_price {
            self.state.highest_market_price = price;
        }
    }

    fn build_child(&self, qty: Quantity) -> ChildOrderRequest {
        ChildOrderRequest {
            parent_algo_id: self.state.id,
            action: ChildOrderAction::Place(OrderRequest {
                symbol: self.state.parent_signal.symbol,
                side: self.state.parent_signal.kind.side(),
                order_type: OrderType::Market,
                quantity: qty,
                price: None,
                trigger_price: None,
                time_in_force: None,
                client_order_id: Some(format!("trailing-{}", self.state.id)),
                take_profit: None,
                stop_loss: None,
                display_quantity: None,
            }),
        }
    }
}

impl ExecutionAlgorithm for TrailingStopAlgorithm {
    fn kind(&self) -> &'static str {
        "TRAILING_STOP"
    }

    fn id(&self) -> &Uuid {
        &self.state.id
    }

    fn status(&self) -> AlgoStatus {
        match self.state.status.as_str() {
            "Working" => AlgoStatus::Working,
            "Completed" => AlgoStatus::Completed,
            "Cancelled" => AlgoStatus::Cancelled,
            other => AlgoStatus::Failed(other.to_string()),
        }
    }

    fn start(&mut self) -> Result<Vec<ChildOrderRequest>> {
        Ok(Vec::new())
    }

    fn on_child_order_placed(&mut self, _order: &Order) {}

    fn on_fill(&mut self, fill: &Fill) -> Result<Vec<ChildOrderRequest>> {
        self.state.filled_quantity += fill.fill_quantity;
        if self.remaining() <= Decimal::ZERO {
            self.state.status = "Completed".into();
        }
        Ok(Vec::new())
    }

    fn on_tick(&mut self, tick: &Tick) -> Result<Vec<ChildOrderRequest>> {
        if !matches!(self.status(), AlgoStatus::Working) {
            return Ok(Vec::new());
        }

        if !self.state.activated {
            self.try_activate(tick.price);
            return Ok(Vec::new());
        }

        if self.state.triggered {
            return Ok(Vec::new());
        }

        self.update_trail(tick.price);
        let threshold = self.state.highest_market_price * (Decimal::ONE - self.state.callback_rate);
        if tick.price <= threshold {
            self.state.triggered = true;
            let qty = self.remaining();
            if qty > Decimal::ZERO {
                return Ok(vec![self.build_child(qty)]);
            }
        }
        Ok(Vec::new())
    }

    fn on_timer(&mut self) -> Result<Vec<ChildOrderRequest>> {
        Ok(Vec::new())
    }

    fn cancel(&mut self) -> Result<()> {
        self.state.status = "Cancelled".into();
        Ok(())
    }

    fn state(&self) -> serde_json::Value {
        serde_json::to_value(&self.state).expect("trailing stop state serialization failed")
    }

    fn from_state(state: serde_json::Value) -> Result<Self>
    where
        Self: Sized,
    {
        let state: TrailingStopState = serde_json::from_value(state)?;
        Ok(Self { state })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tesser_core::SignalKind;

    fn tick(price: Price) -> Tick {
        Tick {
            symbol: "BTCUSDT".into(),
            price,
            size: Decimal::ONE,
            side: tesser_core::Side::Buy,
            exchange_timestamp: Utc::now(),
            received_at: Utc::now(),
        }
    }

    #[test]
    fn trailing_stop_requires_activation() {
        let signal = Signal::new("BTCUSDT", SignalKind::ExitLong, 1.0);
        let mut algo = TrailingStopAlgorithm::new(
            signal,
            Decimal::from(2),
            Decimal::from(100),
            Decimal::new(5, 2),
        )
        .unwrap();
        let orders = algo.on_tick(&tick(Decimal::from(95))).unwrap();
        assert!(orders.is_empty());
        assert!(!algo.state.activated);
        let _ = algo.on_tick(&tick(Decimal::from(101))).unwrap();
        assert!(algo.state.activated);
    }

    #[test]
    fn trailing_stop_triggers_after_callback() {
        let signal = Signal::new("BTCUSDT", SignalKind::ExitLong, 1.0);
        let mut algo = TrailingStopAlgorithm::new(
            signal,
            Decimal::from(3),
            Decimal::from(100),
            Decimal::new(5, 2),
        )
        .unwrap();
        // Activate and push to a new high
        algo.on_tick(&tick(Decimal::from(105))).unwrap();
        algo.on_tick(&tick(Decimal::from(112))).unwrap();
        // Drop below the trailing threshold (112 * (1 - 0.05) = 106.4)
        let orders = algo.on_tick(&tick(Decimal::from(105))).unwrap();
        assert_eq!(orders.len(), 1);
        match &orders[0].action {
            ChildOrderAction::Place(request) => {
                assert_eq!(request.order_type, OrderType::Market);
                assert_eq!(request.quantity, Decimal::from(3));
                assert!(request
                    .client_order_id
                    .as_ref()
                    .expect("client id missing")
                    .starts_with("trailing-"));
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }
}
