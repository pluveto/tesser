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
use tesser_core::{AssetId, Fill, Order, OrderStatus};
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
        let mut canceled = Vec::new();
        let mut filled = Vec::new();
        for order in ghosts {
            warn!(
                order_id = %order.id,
                symbol = %order.request.symbol.code(),
                status = ?order.status,
                "ghost order detected (missing on exchange)"
            );
            let fills = match self
                .client
                .list_order_fills(&order.id, order.request.symbol)
                .await
            {
                Ok(fills) => fills,
                Err(err) => {
                    warn!(
                        order_id = %order.id,
                        symbol = %order.request.symbol.code(),
                        error = %err,
                        "failed to fetch fills for ghost order"
                    );
                    Vec::new()
                }
            };
            if !fills.is_empty() {
                self.metrics
                    .inc_reconciliation_action("ghost_filled", fills.len() as u64);
                self.oms.apply_fills(fills.clone()).await;
                filled.push(build_filled_update(order, &fills));
                continue;
            }
            if matches!(
                order.status,
                OrderStatus::Canceled | OrderStatus::Filled | OrderStatus::Rejected
            ) {
                continue;
            }
            let mut synthetic = order.clone();
            synthetic.status = OrderStatus::Canceled;
            synthetic.updated_at = Utc::now();
            canceled.push(synthetic);
        }
        if !canceled.is_empty() {
            self.metrics
                .inc_reconciliation_action("ghost_canceled", canceled.len() as u64);
            self.oms.apply_order_updates(canceled).await;
        }
        if !filled.is_empty() {
            self.metrics
                .inc_reconciliation_action("ghost_updates", filled.len() as u64);
            self.oms.apply_order_updates(filled).await;
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
        self.metrics
            .inc_reconciliation_action("zombie_adopted", zombies.len() as u64);
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
            self.metrics
                .inc_reconciliation_action("zombie_canceled", canceled.len() as u64);
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

fn build_filled_update(order: &Order, fills: &[Fill]) -> Order {
    let mut synthetic = order.clone();
    let total_qty = fills
        .iter()
        .fold(Decimal::ZERO, |acc, fill| acc + fill.fill_quantity);
    if total_qty > Decimal::ZERO {
        let total_value = fills.iter().fold(Decimal::ZERO, |acc, fill| {
            acc + fill.fill_price * fill.fill_quantity
        });
        synthetic.avg_fill_price = Some(total_value / total_qty);
    }
    if let Some(last) = fills.last() {
        synthetic.updated_at = last.timestamp;
    } else {
        synthetic.updated_at = Utc::now();
    }
    synthetic.status = OrderStatus::Filled;
    synthetic.filled_quantity = total_qty;
    synthetic
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reconcile::snapshot::{ExchangeSnapshot, LocalSnapshot};
    use crate::{
        alerts::{AlertDispatcher, AlertManager},
        live::{OmsHandle, OmsRequest},
        reconcile::OrderDiff,
        telemetry::LiveMetrics,
    };
    use async_trait::async_trait;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tesser_broker::{BrokerError, BrokerInfo, BrokerResult};
    use tesser_config::AlertingConfig;
    use tesser_core::{
        AccountBalance, Instrument, OrderId, OrderRequest, OrderType, OrderUpdateRequest, Position,
        Side, Symbol,
    };
    use tokio::sync::{mpsc, Mutex};
    use tokio::task::JoinHandle;
    use uuid::Uuid;

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

    #[tokio::test]
    async fn runtime_handler_replays_ghost_fills() {
        let harness = TestOmsHarness::new();
        let fake_client = Arc::new(FakeExecutionClient::with_fills(HashMap::from([(
            "ghost-1".to_string(),
            vec![sample_fill(Side::Buy, 1000, 1)],
        )])));
        let handler = runtime_handler_for_tests(harness.handle(), fake_client.clone());
        let order = sample_order("ghost-1", "BTCUSDT");
        let report = ReconciliationReport {
            order_diff: OrderDiff {
                ghosts: vec![order],
                ..Default::default()
            },
            ..Default::default()
        };
        handler.handle(&report).await.unwrap();
        {
            let fills = harness.state.fills.lock().await;
            assert_eq!(fills.len(), 1);
        }
        {
            let orders = harness.state.orders.lock().await;
            assert_eq!(orders.len(), 1);
            assert_eq!(orders[0].status, OrderStatus::Filled);
        }
        harness.shutdown().await;
    }

    #[tokio::test]
    async fn runtime_handler_cancels_zombie_orders() {
        let harness = TestOmsHarness::new();
        let fake_client = Arc::new(FakeExecutionClient::default());
        let handler = runtime_handler_for_tests(harness.handle(), fake_client.clone());
        let order = sample_order("remote-1", "BTCUSDT");
        let report = ReconciliationReport {
            order_diff: OrderDiff {
                zombies: vec![order.clone()],
                ..Default::default()
            },
            ..Default::default()
        };
        handler.handle(&report).await.unwrap();
        {
            let orders = harness.state.orders.lock().await;
            assert!(!orders.is_empty());
            assert_eq!(orders.last().unwrap().status, OrderStatus::Canceled);
        }
        assert_eq!(
            fake_client.canceled().await,
            vec![(order.id.clone(), order.request.symbol)]
        );
        harness.shutdown().await;
    }

    fn runtime_handler_for_tests(
        oms: OmsHandle,
        client: Arc<FakeExecutionClient>,
    ) -> RuntimeHandler {
        let alerts = Arc::new(AlertManager::new(
            AlertingConfig::default(),
            AlertDispatcher::new(None),
            None,
            None,
        ));
        let metrics = Arc::new(LiveMetrics::new());
        RuntimeHandler::new(RuntimeHandlerConfig {
            alerts,
            metrics,
            oms,
            reporting_currency: AssetId::from("USDT"),
            threshold: Decimal::new(1, 3),
            client,
        })
    }

    #[derive(Default)]
    struct TestOmsState {
        orders: Mutex<Vec<Order>>,
        fills: Mutex<Vec<Fill>>,
        liquidate_only: AtomicBool,
    }

    struct TestOmsHarness {
        handle: OmsHandle,
        state: Arc<TestOmsState>,
        task: JoinHandle<()>,
    }

    impl TestOmsHarness {
        fn new() -> Self {
            let (tx, mut rx) = mpsc::channel(16);
            let state = Arc::new(TestOmsState::default());
            let state_handle = state.clone();
            let task = tokio::spawn(async move {
                while let Some(request) = rx.recv().await {
                    match request {
                        OmsRequest::ApplyOrderUpdates { orders, respond_to } => {
                            {
                                let mut guard = state_handle.orders.lock().await;
                                guard.extend(orders);
                            }
                            let _ = respond_to.send(());
                        }
                        OmsRequest::ApplyFills { fills, respond_to } => {
                            {
                                let mut guard = state_handle.fills.lock().await;
                                guard.extend(fills);
                            }
                            let _ = respond_to.send(());
                        }
                        OmsRequest::EnterLiquidateOnly { respond_to } => {
                            state_handle.liquidate_only.store(true, Ordering::SeqCst);
                            let _ = respond_to.send(true);
                        }
                        _ => {}
                    }
                }
            });
            let handle = OmsHandle::new(tx);
            Self {
                handle,
                state,
                task,
            }
        }

        fn handle(&self) -> OmsHandle {
            self.handle.clone()
        }

        async fn shutdown(self) {
            self.task.abort();
        }
    }

    #[derive(Default)]
    struct FakeExecutionClient {
        fills: Mutex<HashMap<String, Vec<Fill>>>,
        canceled: Mutex<Vec<(String, Symbol)>>,
    }

    impl FakeExecutionClient {
        fn with_fills(map: HashMap<String, Vec<Fill>>) -> Self {
            Self {
                fills: Mutex::new(map),
                canceled: Mutex::new(Vec::new()),
            }
        }

        async fn canceled(&self) -> Vec<(String, Symbol)> {
            self.canceled.lock().await.clone()
        }
    }

    #[async_trait]
    impl ExecutionClient for FakeExecutionClient {
        fn info(&self) -> BrokerInfo {
            BrokerInfo {
                name: "test".into(),
                markets: vec![],
                supports_testnet: true,
            }
        }

        async fn place_order(&self, _request: OrderRequest) -> BrokerResult<Order> {
            Err(BrokerError::Other("not implemented".into()))
        }

        async fn cancel_order(&self, order_id: OrderId, symbol: Symbol) -> BrokerResult<()> {
            let mut guard = self.canceled.lock().await;
            guard.push((order_id, symbol));
            Ok(())
        }

        async fn amend_order(&self, _request: OrderUpdateRequest) -> BrokerResult<Order> {
            Err(BrokerError::Other("not implemented".into()))
        }

        async fn list_open_orders(&self, _symbol: Symbol) -> BrokerResult<Vec<Order>> {
            Ok(Vec::new())
        }

        async fn account_balances(&self) -> BrokerResult<Vec<AccountBalance>> {
            Ok(Vec::new())
        }

        async fn positions(&self, _symbols: Option<&Vec<Symbol>>) -> BrokerResult<Vec<Position>> {
            Ok(Vec::new())
        }

        async fn list_instruments(&self, _category: &str) -> BrokerResult<Vec<Instrument>> {
            Ok(Vec::new())
        }

        async fn list_order_fills(
            &self,
            order_id: &str,
            _symbol: Symbol,
        ) -> BrokerResult<Vec<Fill>> {
            let guard = self.fills.lock().await;
            Ok(guard.get(order_id).cloned().unwrap_or_default())
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    fn sample_fill(side: Side, price: i64, qty: i64) -> Fill {
        Fill {
            order_id: Uuid::new_v4().to_string(),
            symbol: Symbol::from("BTCUSDT"),
            side,
            fill_price: Decimal::new(price, 0),
            fill_quantity: Decimal::new(qty, 0),
            fee: None,
            fee_asset: None,
            timestamp: Utc::now(),
        }
    }
}
