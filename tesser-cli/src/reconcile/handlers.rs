use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use tracing::{error, info, warn};

use crate::alerts::AlertManager;
use crate::live::OmsHandle;
use crate::telemetry::LiveMetrics;

use super::diff::{BalanceDiscrepancy, PositionDiscrepancy, ReconciliationReport};
use super::snapshot::{ExchangeSnapshot, LocalSnapshot};
use super::StateDiffer;
use tesser_broker::ExecutionClient;
use tesser_core::{AssetId, Order, OrderStatus};
use tesser_markets::MarketRegistry;
use tesser_portfolio::{Portfolio, PortfolioConfig, PortfolioState};

/// Configuration for the runtime handler.
#[derive(Clone)]
pub struct RuntimeHandlerConfig {
    pub alerts: Arc<AlertManager>,
    pub metrics: Arc<LiveMetrics>,
    pub oms: OmsHandle,
    pub reporting_currency: AssetId,
    pub threshold: Decimal,
    pub client: Arc<dyn ExecutionClient>,
}

/// Applies fine-grained corrections during the live reconciliation loop.
pub struct RuntimeHandler {
    alerts: Arc<AlertManager>,
    metrics: Arc<LiveMetrics>,
    oms: OmsHandle,
    reporting_currency: AssetId,
    threshold: Decimal,
    client: Arc<dyn ExecutionClient>,
}

impl RuntimeHandler {
    pub fn new(config: RuntimeHandlerConfig) -> Self {
        Self {
            alerts: config.alerts,
            metrics: config.metrics,
            oms: config.oms,
            reporting_currency: config.reporting_currency,
            threshold: if config.threshold <= Decimal::ZERO {
                Decimal::new(1, 6)
            } else {
                config.threshold
            },
            client: config.client,
        }
    }

    pub async fn handle(&self, report: &ReconciliationReport) -> Result<()> {
        let mut severe_findings = Vec::new();
        self.handle_positions(&report.position_diff.discrepancies, &mut severe_findings);
        self.handle_balances(&report.balance_diff.discrepancies, &mut severe_findings);
        self.resolve_ghost_orders(&report.order_diff.ghosts).await;
        self.resolve_zombie_orders(&report.order_diff.zombies).await;

        if severe_findings.is_empty() {
            info!("state reconciliation complete with no critical divergence");
            return Ok(());
        }

        let alert_body = severe_findings.join("; ");
        self.alerts
            .notify("State reconciliation divergence", &alert_body)
            .await;
        self.oms.enter_liquidate_only().await;
        Ok(())
    }

    fn handle_positions(&self, entries: &[PositionDiscrepancy], severe: &mut Vec<String>) {
        for entry in entries {
            let diff = entry.delta.abs();
            let symbol_label = entry.symbol.code().to_string();
            self.metrics
                .update_position_diff(&symbol_label, diff.to_f64().unwrap_or(0.0));
            if diff.is_zero() {
                continue;
            }
            warn!(
                symbol = %symbol_label,
                local = %entry.local_signed,
                remote = %entry.remote_signed,
                diff = %diff,
                "position mismatch detected during reconciliation"
            );
            let pct = normalize_diff(diff, entry.remote_signed);
            if pct >= self.threshold {
                error!(
                    symbol = %symbol_label,
                    local = %entry.local_signed,
                    remote = %entry.remote_signed,
                    diff = %diff,
                    pct = %pct,
                    "position mismatch exceeds threshold"
                );
                severe.push(format!(
                    "{symbol_label} local={} remote={} diff={diff}",
                    entry.local_signed, entry.remote_signed
                ));
            }
        }
    }

    fn handle_balances(&self, entries: &[BalanceDiscrepancy], severe: &mut Vec<String>) {
        let reporting = self.reporting_currency;
        let label = reporting.to_string();
        let entry = entries.iter().find(|entry| entry.asset == reporting);
        let (local_cash, remote_cash) = entry
            .map(|record| {
                (
                    record.local_available.unwrap_or(Decimal::ZERO),
                    record.remote_available.unwrap_or(Decimal::ZERO),
                )
            })
            .unwrap_or((Decimal::ZERO, Decimal::ZERO));
        let diff = (local_cash - remote_cash).abs();
        self.metrics
            .update_balance_diff(&label, diff.to_f64().unwrap_or(0.0));
        if diff.is_zero() {
            return;
        }
        warn!(
            currency = %label,
            local = %local_cash,
            remote = %remote_cash,
            diff = %diff,
            "balance mismatch detected during reconciliation"
        );
        let pct = normalize_diff(diff, remote_cash);
        if pct >= self.threshold {
            error!(
                currency = %label,
                local = %local_cash,
                remote = %remote_cash,
                diff = %diff,
                pct = %pct,
                "balance mismatch exceeds threshold"
            );
            severe.push(format!(
                "{label} balance local={local_cash} remote={remote_cash} diff={diff}"
            ));
        }
    }

    async fn resolve_ghost_orders(&self, ghosts: &[Order]) {
        if ghosts.is_empty() {
            return;
        }
        let mut updates = Vec::new();
        for order in ghosts {
            warn!(
                order_id = %order.id,
                symbol = %order.request.symbol.code(),
                status = ?order.status,
                "ghost order detected (missing on exchange)"
            );
            if matches!(
                order.status,
                OrderStatus::Canceled | OrderStatus::Filled | OrderStatus::Rejected
            ) {
                continue;
            }
            let mut synthetic = order.clone();
            synthetic.status = OrderStatus::Canceled;
            synthetic.updated_at = Utc::now();
            updates.push(synthetic);
        }
        if !updates.is_empty() {
            self.oms.apply_order_updates(updates).await;
        }
    }

    async fn resolve_zombie_orders(&self, zombies: &[Order]) {
        if zombies.is_empty() {
            return;
        }
        for order in zombies {
            warn!(
                order_id = %order.id,
                symbol = %order.request.symbol.code(),
                status = ?order.status,
                "zombie order detected (present on exchange but unknown locally)"
            );
        }
        // Adopt remote state before attempting any cancellations so the OMS is aware of them.
        self.oms.apply_order_updates(zombies.to_vec()).await;
        let mut canceled = Vec::new();
        for order in zombies {
            match self
                .client
                .cancel_order(order.id.clone(), order.request.symbol)
                .await
            {
                Ok(_) => {
                    let mut update = order.clone();
                    update.status = OrderStatus::Canceled;
                    update.updated_at = Utc::now();
                    canceled.push(update);
                }
                Err(err) => {
                    warn!(
                        order_id = %order.id,
                        symbol = %order.request.symbol.code(),
                        error = %err,
                        "failed to cancel zombie order during reconciliation"
                    );
                }
            }
        }
        if !canceled.is_empty() {
            self.oms.apply_order_updates(canceled).await;
        }
    }
}

/// Configuration for the startup handler.
pub struct StartupHandlerConfig {
    pub portfolio_config: PortfolioConfig,
    pub market_registry: Arc<MarketRegistry>,
}

/// Applies coarse-grained corrections during startup.
pub struct StartupHandler {
    portfolio_config: PortfolioConfig,
    market_registry: Arc<MarketRegistry>,
}

impl StartupHandler {
    pub fn new(config: StartupHandlerConfig) -> Self {
        Self {
            portfolio_config: config.portfolio_config,
            market_registry: config.market_registry,
        }
    }

    pub fn reconcile(
        &self,
        local_state: LocalSnapshot,
        remote_state: ExchangeSnapshot,
        preserved_metrics: Option<&PortfolioState>,
    ) -> StartupOutcome {
        let report = StateDiffer::diff(local_state, remote_state);
        self.apply(&report, preserved_metrics)
    }

    pub fn apply(
        &self,
        report: &ReconciliationReport,
        preserved_metrics: Option<&PortfolioState>,
    ) -> StartupOutcome {
        let portfolio = Portfolio::from_exchange_state(
            report.remote.positions.clone(),
            report.remote.balances.clone(),
            self.portfolio_config.clone(),
            self.market_registry.clone(),
            preserved_metrics,
        );
        if let Some(diff) = report.position_diff.discrepancies.first() {
            info!(
                symbol = %diff.symbol.code(),
                local = %diff.local_signed,
                remote = %diff.remote_signed,
                "position divergence detected during startup"
            );
        }
        if !report.order_diff.ghosts.is_empty() {
            for order in &report.order_diff.ghosts {
                info!(
                    order_id = %order.id,
                    symbol = %order.request.symbol.code(),
                    "dropping ghost order from local state during startup"
                );
            }
        }
        if !report.order_diff.zombies.is_empty() {
            for order in &report.order_diff.zombies {
                info!(
                    order_id = %order.id,
                    symbol = %order.request.symbol.code(),
                    "adopting remote zombie order during startup reconciliation"
                );
            }
        }
        StartupOutcome {
            portfolio,
            open_orders: report.remote.open_orders.clone(),
            cancel_orders: report.order_diff.zombies.clone(),
        }
    }
}

/// Result of applying the startup handler.
pub struct StartupOutcome {
    pub portfolio: Portfolio,
    pub open_orders: Vec<Order>,
    pub cancel_orders: Vec<Order>,
}

fn normalize_diff(diff: Decimal, reference: Decimal) -> Decimal {
    if diff <= Decimal::ZERO {
        Decimal::ZERO
    } else {
        let denominator = std::cmp::max(reference.abs(), Decimal::ONE);
        diff / denominator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::snapshot::{ExchangeSnapshot, LocalSnapshot};
    use chrono::Utc;
    use rust_decimal::Decimal;
    use tesser_core::{OrderRequest, OrderType, Side, Symbol};

    fn sample_order(id: &str, symbol: &str) -> Order {
        Order {
            id: id.to_string(),
            request: OrderRequest {
                symbol: Symbol::from(symbol),
                side: Side::Buy,
                order_type: OrderType::Limit,
                quantity: Decimal::ONE,
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

    #[test]
    fn startup_marks_remote_zombies_for_cancellation() {
        let handler = StartupHandler::new(StartupHandlerConfig {
            portfolio_config: PortfolioConfig::default(),
            market_registry: Arc::new(MarketRegistry::default()),
        });
        let local = LocalSnapshot::new(None, vec![sample_order("local", "BTCUSDT")]);
        let remote = ExchangeSnapshot::new(
            Vec::new(),
            Vec::new(),
            vec![sample_order("remote", "BTCUSDT")],
        );
        let outcome = handler.reconcile(local, remote, None);
        assert_eq!(outcome.cancel_orders.len(), 1);
        assert_eq!(outcome.cancel_orders[0].id, "remote");
    }
}
