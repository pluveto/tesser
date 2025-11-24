#![cfg(feature = "bybit")]

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use tesser_broker::{ExecutionClient, Quota};
use tesser_bybit::{BybitClient, BybitConfig, BybitCredentials};
use tesser_core::{
    AccountBalance, AssetId, Candle, ExchangeId, Interval, OrderRequest, Side, Symbol, Tick,
};
use tesser_test_utils::{AccountConfig, MockExchange, MockExchangeConfig};

const SYMBOL: &str = "BTCUSDT";

fn bybit_exchange() -> ExchangeId {
    ExchangeId::from("bybit_linear")
}

fn test_symbol() -> Symbol {
    Symbol::from_code(bybit_exchange(), SYMBOL)
}

fn usdt_asset() -> AssetId {
    AssetId::from_code(bybit_exchange(), "USDT")
}

#[tokio::test(flavor = "multi_thread")]
async fn bybit_client_honors_private_rate_limit() -> Result<()> {
    let usdt = usdt_asset();
    let account = AccountConfig::new("limit-key", "limit-secret").with_balance(AccountBalance {
        exchange: usdt.exchange,
        asset: usdt,
        total: Decimal::new(10_000, 0),
        available: Decimal::new(10_000, 0),
        updated_at: Utc::now(),
    });
    let candles = vec![Candle {
        symbol: test_symbol(),
        interval: Interval::OneMinute,
        open: Decimal::new(20_000, 0),
        high: Decimal::new(20_010, 0),
        low: Decimal::new(19_990, 0),
        close: Decimal::new(20_005, 0),
        volume: Decimal::ONE,
        timestamp: Utc::now(),
    }];
    let ticks = vec![Tick {
        symbol: test_symbol(),
        price: Decimal::new(20_005, 0),
        size: Decimal::ONE,
        side: Side::Buy,
        exchange_timestamp: Utc::now(),
        received_at: Utc::now(),
    }];
    let config = MockExchangeConfig::new()
        .with_exchange(bybit_exchange())
        .with_account(account)
        .with_candles(candles)
        .with_ticks(ticks);
    let mut exchange = MockExchange::start(config).await?;

    let client_cfg = BybitConfig {
        base_url: exchange.rest_url(),
        ws_url: Some(exchange.ws_url()),
        private_quota: NonZeroU32::new(1).map(Quota::per_second),
        ..BybitConfig::default()
    };
    let client = BybitClient::new(
        client_cfg,
        Some(BybitCredentials {
            api_key: "limit-key".into(),
            api_secret: "limit-secret".into(),
        }),
        ExchangeId::from("bybit_linear"),
    );

    let request = OrderRequest {
        symbol: test_symbol(),
        side: Side::Buy,
        order_type: tesser_core::OrderType::Market,
        quantity: Decimal::new(1, 0),
        price: None,
        trigger_price: None,
        time_in_force: None,
        client_order_id: None,
        take_profit: None,
        stop_loss: None,
        display_quantity: None,
    };

    let start = Instant::now();
    for _ in 0..3 {
        client.place_order(request.clone()).await?;
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_secs(2),
        "expected rate limiter to enforce serialized requests, elapsed {:?}",
        elapsed
    );

    exchange.shutdown().await;
    Ok(())
}
