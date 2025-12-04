use tesser_core::{AccountBalance, Order, Position};
use tesser_portfolio::{LiveState, PortfolioState};

/// Lightweight clone of the OMS state used for reconciliation.
#[derive(Clone, Debug, Default)]
pub struct LocalSnapshot {
    pub portfolio: Option<PortfolioState>,
    pub open_orders: Vec<Order>,
}

impl LocalSnapshot {
    /// Build a snapshot from persisted OMS state.
    pub fn new(portfolio: Option<PortfolioState>, open_orders: Vec<Order>) -> Self {
        Self {
            portfolio,
            open_orders,
        }
    }

    /// Convert a [`LiveState`] instance into a reconciliation snapshot.
    pub fn from_live_state(state: &LiveState) -> Self {
        Self {
            portfolio: state.portfolio.clone(),
            open_orders: state.open_orders.clone(),
        }
    }
}

/// Remote exchange snapshot captured via REST calls.
#[derive(Clone, Debug, Default)]
pub struct ExchangeSnapshot {
    pub positions: Vec<Position>,
    pub balances: Vec<AccountBalance>,
    pub open_orders: Vec<Order>,
}

impl ExchangeSnapshot {
    /// Helper constructor for remote snapshots.
    pub fn new(
        positions: Vec<Position>,
        balances: Vec<AccountBalance>,
        open_orders: Vec<Order>,
    ) -> Self {
        Self {
            positions,
            balances,
            open_orders,
        }
    }
}
