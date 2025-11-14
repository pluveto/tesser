use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use tesser_core::{Candle, Order, Price, Side};

/// Internal classification for conditional orders (used for OCO resolution).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum TriggerKind {
    Standalone,
    StopLoss,
    TakeProfit,
}

impl TriggerKind {
    fn priority(self) -> u8 {
        match self {
            Self::StopLoss => 0,
            Self::TakeProfit => 1,
            Self::Standalone => 2,
        }
    }
}

/// Triggered order metadata returned by the manager.
pub struct TriggeredOrder {
    pub order: Order,
    pub fill_price: Price,
    pub timestamp: DateTime<Utc>,
    pub kind: TriggerKind,
    pub group: Option<String>,
}

struct PendingConditional {
    order: Order,
    kind: TriggerKind,
    group: Option<String>,
}

/// Maintains a queue of conditional orders (stop-loss, take-profit, etc.).
#[derive(Default)]
pub struct ConditionalOrderManager {
    orders: Vec<PendingConditional>,
}

impl ConditionalOrderManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a conditional order so it may be triggered later.
    pub fn push(&mut self, order: Order) {
        let (group, kind) = parse_group(&order);
        self.orders.push(PendingConditional { order, kind, group });
    }

    /// Trigger any orders touched by the provided candle range.
    pub fn trigger_with_candle(&mut self, candle: &Candle) -> Vec<TriggeredOrder> {
        self.evaluate(|pending| {
            let trigger = pending.order.request.trigger_price?;
            let touched = match pending.order.request.side {
                Side::Buy => candle.high >= trigger,
                Side::Sell => candle.low <= trigger,
            };
            touched.then_some((trigger, candle.timestamp))
        })
    }

    /// Trigger any orders whose thresholds were crossed by the provided trade price.
    pub fn trigger_with_price(
        &mut self,
        last_price: Price,
        timestamp: DateTime<Utc>,
    ) -> Vec<TriggeredOrder> {
        self.evaluate(|pending| {
            let trigger = pending.order.request.trigger_price?;
            let touched = match pending.order.request.side {
                Side::Buy => last_price >= trigger,
                Side::Sell => last_price <= trigger,
            };
            touched.then_some((last_price, timestamp))
        })
    }

    fn evaluate<F>(&mut self, evaluator: F) -> Vec<TriggeredOrder>
    where
        F: Fn(&PendingConditional) -> Option<(Price, DateTime<Utc>)>,
    {
        let mut survivors = Vec::with_capacity(self.orders.len());
        let mut triggered = Vec::new();
        for pending in self.orders.drain(..) {
            if let Some((price, ts)) = evaluator(&pending) {
                triggered.push(TriggeredOrder {
                    order: pending.order,
                    fill_price: price,
                    timestamp: ts,
                    kind: pending.kind,
                    group: pending.group.clone(),
                });
            } else {
                survivors.push(pending);
            }
        }

        let mut drop_groups = HashSet::new();
        let mut resolved = Vec::new();
        let mut grouped: HashMap<String, Vec<TriggeredOrder>> = HashMap::new();
        for event in triggered.into_iter() {
            if let Some(group) = &event.group {
                grouped.entry(group.clone()).or_default().push(event);
            } else {
                resolved.push(event);
            }
        }

        for (group, mut events) in grouped.into_iter() {
            events.sort_by_key(|event| event.kind.priority());
            if let Some(best) = events.into_iter().next() {
                drop_groups.insert(group);
                resolved.push(best);
            }
        }

        self.orders = survivors
            .into_iter()
            .filter(|pending| {
                pending
                    .group
                    .as_ref()
                    .map(|group| !drop_groups.contains(group))
                    .unwrap_or(true)
            })
            .collect();

        resolved
    }
}

fn parse_group(order: &Order) -> (Option<String>, TriggerKind) {
    if let Some(cid) = order.request.client_order_id.as_ref() {
        if let Some(base) = cid.strip_suffix("-sl") {
            return (Some(base.to_string()), TriggerKind::StopLoss);
        }
        if let Some(base) = cid.strip_suffix("-tp") {
            return (Some(base.to_string()), TriggerKind::TakeProfit);
        }
    }
    (None, TriggerKind::Standalone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use tesser_core::{OrderRequest, OrderStatus, Quantity, TimeInForce};
    use uuid::Uuid;

    fn pending(side: Side, trigger: Price, cid: &str) -> Order {
        Order {
            id: Uuid::new_v4().to_string(),
            request: OrderRequest {
                symbol: "BTCUSDT".into(),
                side,
                order_type: tesser_core::OrderType::StopMarket,
                quantity: Quantity::from(1),
                price: None,
                trigger_price: Some(trigger),
                time_in_force: Some(TimeInForce::GoodTilCanceled),
                client_order_id: Some(cid.into()),
                take_profit: None,
                stop_loss: None,
                display_quantity: None,
            },
            status: OrderStatus::PendingNew,
            filled_quantity: Quantity::ZERO,
            avg_fill_price: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn stop_loss_wins_over_take_profit() {
        let mut book = ConditionalOrderManager::new();
        book.push(pending(Side::Sell, Decimal::from(90), "base-sl"));
        book.push(pending(Side::Sell, Decimal::from(110), "base-tp"));
        let candle = Candle {
            symbol: "BTCUSDT".into(),
            interval: tesser_core::Interval::OneMinute,
            open: Decimal::from(100),
            high: Decimal::from(120),
            low: Decimal::from(80),
            close: Decimal::from(95),
            volume: Decimal::from(1),
            timestamp: Utc::now(),
        };
        let triggered = book.trigger_with_candle(&candle);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].kind, TriggerKind::StopLoss);
    }
}
