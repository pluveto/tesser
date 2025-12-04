use std::collections::{HashMap, HashSet};

use rust_decimal::Decimal;
use tesser_core::{AssetId, Order, Position, Side, Symbol};

use super::snapshot::{ExchangeSnapshot, LocalSnapshot};

/// Unified report describing every divergence detected between local and remote state.
#[derive(Clone, Debug, Default)]
pub struct ReconciliationReport {
    pub local: LocalSnapshot,
    pub remote: ExchangeSnapshot,
    pub order_diff: OrderDiff,
    pub position_diff: PositionDiff,
    pub balance_diff: BalanceDiff,
}

/// Stateless engine that compares local and remote snapshots.
pub struct StateDiffer;

impl StateDiffer {
    /// Compute a reconciliation report for the provided snapshots.
    pub fn diff(local: LocalSnapshot, remote: ExchangeSnapshot) -> ReconciliationReport {
        let order_diff = Self::diff_orders(&local.open_orders, &remote.open_orders);
        let position_diff = Self::diff_positions(&local, &remote);
        let balance_diff = Self::diff_balances(&local, &remote);
        ReconciliationReport {
            local,
            remote,
            order_diff,
            position_diff,
            balance_diff,
        }
    }

    fn diff_orders(local: &[Order], remote: &[Order]) -> OrderDiff {
        let mut remote_index: HashMap<String, Order> = HashMap::new();
        for order in remote {
            remote_index.insert(order.id.clone(), order.clone());
        }
        let mut matched = Vec::new();
        let mut ghosts = Vec::new();
        for local_order in local {
            if let Some(remote_order) = remote_index.remove(&local_order.id) {
                matched.push(OrderPair {
                    local: local_order.clone(),
                    remote: remote_order,
                });
            } else {
                ghosts.push(local_order.clone());
            }
        }
        let zombies = remote_index.into_values().collect();
        OrderDiff {
            matched,
            ghosts,
            zombies,
        }
    }

    fn diff_positions(local: &LocalSnapshot, remote: &ExchangeSnapshot) -> PositionDiff {
        let mut discrepancies = Vec::new();
        let mut symbols: HashSet<Symbol> = HashSet::new();
        if let Some(portfolio) = &local.portfolio {
            symbols.extend(portfolio.positions.keys().copied());
        }
        for position in &remote.positions {
            symbols.insert(position.symbol);
        }
        let remote_index: HashMap<Symbol, Position> = remote
            .positions
            .iter()
            .cloned()
            .map(|pos| (pos.symbol, pos))
            .collect();
        for symbol in symbols {
            let local_position = local
                .portfolio
                .as_ref()
                .and_then(|portfolio| portfolio.positions.get(&symbol))
                .cloned();
            let remote_position = remote_index.get(&symbol).cloned();
            let local_signed = local_position
                .as_ref()
                .map(signed_quantity)
                .unwrap_or(Decimal::ZERO);
            let remote_signed = remote_position
                .as_ref()
                .map(signed_quantity)
                .unwrap_or(Decimal::ZERO);
            if local_signed == remote_signed {
                continue;
            }
            discrepancies.push(PositionDiscrepancy {
                symbol,
                local: local_position,
                remote: remote_position,
                local_signed,
                remote_signed,
                delta: local_signed - remote_signed,
            });
        }
        PositionDiff { discrepancies }
    }

    fn diff_balances(local: &LocalSnapshot, remote: &ExchangeSnapshot) -> BalanceDiff {
        let mut discrepancies = Vec::new();
        let mut assets: HashSet<AssetId> = HashSet::new();
        if let Some(portfolio) = &local.portfolio {
            assets.extend(portfolio.balances.iter().map(|(asset, _)| *asset));
        }
        for balance in &remote.balances {
            assets.insert(balance.asset);
        }
        let remote_index: HashMap<AssetId, tesser_core::AccountBalance> = remote
            .balances
            .iter()
            .cloned()
            .map(|balance| (balance.asset, balance))
            .collect();
        for asset in assets {
            let local_cash = local
                .portfolio
                .as_ref()
                .and_then(|portfolio| portfolio.balances.get(asset))
                .cloned();
            let remote_cash = remote_index.get(&asset).cloned();
            let local_value = local_cash
                .as_ref()
                .map(|cash| cash.quantity)
                .unwrap_or(Decimal::ZERO);
            let remote_value = remote_cash
                .as_ref()
                .map(|balance| balance.available)
                .unwrap_or(Decimal::ZERO);
            if local_value == remote_value {
                continue;
            }
            let delta = local_value - remote_value;
            if delta.is_zero() {
                continue;
            }
            discrepancies.push(BalanceDiscrepancy {
                asset,
                local_available: local_cash.map(|cash| cash.quantity),
                remote_available: remote_cash.as_ref().map(|balance| balance.available),
                delta,
            });
        }
        BalanceDiff { discrepancies }
    }
}

/// Order comparison summary.
#[derive(Clone, Debug, Default)]
pub struct OrderDiff {
    pub matched: Vec<OrderPair>,
    pub ghosts: Vec<Order>,
    pub zombies: Vec<Order>,
}

/// Matched order pair between local and remote sources.
#[derive(Clone, Debug)]
pub struct OrderPair {
    pub local: Order,
    pub remote: Order,
}

/// Aggregate view of position deltas.
#[derive(Clone, Debug, Default)]
pub struct PositionDiff {
    pub discrepancies: Vec<PositionDiscrepancy>,
}

/// Detail for a single position mismatch.
#[derive(Clone, Debug, Default)]
pub struct PositionDiscrepancy {
    pub symbol: Symbol,
    pub local: Option<Position>,
    pub remote: Option<Position>,
    pub local_signed: Decimal,
    pub remote_signed: Decimal,
    pub delta: Decimal,
}

/// Aggregate view of balance mismatches.
#[derive(Clone, Debug, Default)]
pub struct BalanceDiff {
    pub discrepancies: Vec<BalanceDiscrepancy>,
}

/// Detail for a single currency mismatch.
#[derive(Clone, Debug, Default)]
pub struct BalanceDiscrepancy {
    pub asset: AssetId,
    pub local_available: Option<Decimal>,
    pub remote_available: Option<Decimal>,
    pub delta: Decimal,
}

fn signed_quantity(position: &Position) -> Decimal {
    match position.side {
        Some(Side::Buy) => position.quantity,
        Some(Side::Sell) => -position.quantity,
        None => Decimal::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::prelude::FromPrimitive;
    use tesser_core::{
        AccountBalance, AssetId, Cash, ExchangeId, OrderRequest, OrderStatus, OrderType, Position,
        Side, Symbol,
    };

    fn sample_order(id: &str) -> Order {
        Order {
            id: id.to_string(),
            request: OrderRequest {
                symbol: Symbol::from("BTCUSDT"),
                side: Side::Buy,
                order_type: OrderType::Limit,
                quantity: Decimal::from_f64(1.0).unwrap(),
                price: None,
                trigger_price: None,
                time_in_force: None,
                client_order_id: None,
                take_profit: None,
                stop_loss: None,
                display_quantity: None,
            },
            status: OrderStatus::Accepted,
            filled_quantity: Decimal::ZERO,
            avg_fill_price: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_position(symbol: Symbol, qty: f64, side: Side) -> Position {
        Position {
            symbol,
            side: Some(side),
            quantity: Decimal::from_f64(qty).unwrap(),
            entry_price: None,
            unrealized_pnl: Decimal::ZERO,
            updated_at: Utc::now(),
        }
    }

    fn sample_balance(asset: AssetId, amount: f64) -> AccountBalance {
        AccountBalance {
            exchange: ExchangeId::UNSPECIFIED,
            asset,
            total: Decimal::from_f64(amount).unwrap(),
            available: Decimal::from_f64(amount).unwrap(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn detects_order_ghosts_and_zombies() {
        let local = LocalSnapshot::new(None, vec![sample_order("A"), sample_order("B")]);
        let remote = ExchangeSnapshot::new(
            Vec::new(),
            Vec::new(),
            vec![sample_order("B"), sample_order("C")],
        );
        let report = StateDiffer::diff(local, remote);
        assert_eq!(report.order_diff.ghosts.len(), 1);
        assert_eq!(report.order_diff.zombies.len(), 1);
        assert_eq!(report.order_diff.matched.len(), 1);
        assert_eq!(report.order_diff.ghosts[0].id, "A");
        assert_eq!(report.order_diff.zombies[0].id, "C");
    }

    #[test]
    fn detects_position_delta() {
        let symbol = Symbol::from("BTCUSDT");
        let mut state = tesser_portfolio::PortfolioState::default();
        state
            .positions
            .insert(symbol, sample_position(symbol, 1.0, Side::Buy));
        let local = LocalSnapshot::new(Some(state), Vec::new());
        let remote = ExchangeSnapshot::new(
            vec![sample_position(symbol, 0.5, Side::Buy)],
            Vec::new(),
            Vec::new(),
        );
        let report = StateDiffer::diff(local, remote);
        assert_eq!(report.position_diff.discrepancies.len(), 1);
        assert_eq!(
            report.position_diff.discrepancies[0].delta,
            Decimal::from_f64(0.5).unwrap()
        );
    }

    #[test]
    fn detects_balance_delta() {
        let asset = AssetId::from("USDT");
        let mut state = tesser_portfolio::PortfolioState::default();
        state.balances.upsert(Cash {
            currency: asset,
            quantity: Decimal::from_f64(10.0).unwrap(),
            conversion_rate: Decimal::ONE,
        });
        let local = LocalSnapshot::new(Some(state), Vec::new());
        let remote =
            ExchangeSnapshot::new(Vec::new(), vec![sample_balance(asset, 9.0)], Vec::new());
        let report = StateDiffer::diff(local, remote);
        assert_eq!(report.balance_diff.discrepancies.len(), 1);
        assert_eq!(
            report.balance_diff.discrepancies[0].delta,
            Decimal::from_f64(1.0).unwrap()
        );
    }
}
