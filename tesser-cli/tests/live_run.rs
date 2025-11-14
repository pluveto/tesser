use anyhow::Result;
use chrono::Utc;
use rust_decimal::Decimal;
use tesser_core::{AccountBalance, Candle, Interval, Side, Tick};
use tesser_test_utils::{AccountConfig, MockExchange, MockExchangeConfig};

#[tokio::test(flavor = "multi_thread")]
async fn mock_exchange_starts() -> Result<()> {
    let account = AccountConfig::new("test-key", "test-secret").with_balance(AccountBalance {
        currency: "USDT".into(),
        total: Decimal::new(10000, 0),
        available: Decimal::new(10000, 0),
        updated_at: Utc::now(),
    });

    let candles = vec![Candle {
        symbol: "BTCUSDT".into(),
        interval: Interval::OneMinute,
        open: Decimal::new(1000, 0),
        high: Decimal::new(1010, 0),
        low: Decimal::new(995, 0),
        close: Decimal::new(1005, 0),
        volume: Decimal::new(1, 0),
        timestamp: Utc::now(),
    }];

    let ticks = vec![Tick {
        symbol: "BTCUSDT".into(),
        price: Decimal::new(1005, 0),
        size: Decimal::new(1, 0),
        side: Side::Buy,
        exchange_timestamp: Utc::now(),
        received_at: Utc::now(),
    }];

    let config = MockExchangeConfig::new()
        .with_account(account)
        .with_candles(candles)
        .with_ticks(ticks);
    let exchange = MockExchange::start(config).await?;
    assert!(exchange.rest_url().starts_with("http://127.0.0.1"));
    assert!(exchange.ws_url().starts_with("ws://127.0.0.1"));
    Ok(())
}
