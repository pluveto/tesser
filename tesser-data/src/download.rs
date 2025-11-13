use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;
use tesser_core::{Candle, Interval, Symbol};
use tracing::debug;

const MAX_LIMIT: usize = 1000;

/// Parameters for a kline download request.
pub struct KlineRequest<'a> {
    pub category: &'a str,
    pub symbol: &'a str,
    pub interval: Interval,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub limit: usize,
}

impl<'a> KlineRequest<'a> {
    pub fn new(
        category: &'a str,
        symbol: &'a str,
        interval: Interval,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        Self {
            category,
            symbol,
            interval,
            start,
            end,
            limit: MAX_LIMIT,
        }
    }
}

/// Simple Bybit REST downloader for kline data.
pub struct BybitDownloader {
    client: Client,
    base_url: String,
}

impl BybitDownloader {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{base}/{path}")
    }

    /// Download klines from Bybit, returning a chronologically sorted list of candles.
    pub async fn download_klines(&self, req: &KlineRequest<'_>) -> Result<Vec<Candle>> {
        let mut cursor = req.start.timestamp_millis();
        let end_ms = req.end.timestamp_millis();
        if cursor >= end_ms {
            return Err(anyhow!("start must be earlier than end"));
        }

        let mut candles = Vec::new();
        let interval_ms = req.interval.as_duration().num_milliseconds();

        while cursor < end_ms {
            let limit = req.limit.min(MAX_LIMIT).to_string();
            let response = self
                .client
                .get(self.endpoint("v5/market/kline"))
                .query(&[
                    ("category", req.category),
                    ("symbol", req.symbol),
                    ("interval", req.interval.to_bybit()),
                    ("start", &cursor.to_string()),
                    ("end", &end_ms.to_string()),
                    ("limit", &limit),
                ])
                .send()
                .await
                .context("request to Bybit failed")?;

            let status = response.status();
            let body = response
                .text()
                .await
                .context("failed to read Bybit response body")?;
            debug!(
                "bybit kline response (status {}): {}",
                status,
                truncate(&body, 512)
            );
            if !status.is_success() {
                return Err(anyhow!(
                    "Bybit responded with status {}: {}",
                    status,
                    truncate(&body, 256)
                ));
            }

            let response: BybitKlineResponse = serde_json::from_str(&body).map_err(|err| {
                anyhow!(
                    "failed to parse Bybit response: {} (body snippet: {})",
                    err,
                    truncate(&body, 256)
                )
            })?;

            if response.ret_code != 0 {
                return Err(anyhow!(
                    "Bybit returned error {}: {}",
                    response.ret_code,
                    response.ret_msg
                ));
            }

            let result = match response.result {
                Some(result) => result,
                None => break,
            };
            if result.list.is_empty() {
                break;
            }

            let mut batch = Vec::new();
            for entry in result.list {
                if let Some(candle) = parse_entry(&entry, req.symbol, req.interval) {
                    if candle.timestamp.timestamp_millis() >= cursor
                        && candle.timestamp.timestamp_millis() <= end_ms
                    {
                        batch.push(candle);
                    }
                }
            }

            if batch.is_empty() {
                break;
            }

            batch.sort_by_key(|c| c.timestamp);
            cursor = batch
                .last()
                .map(|c| c.timestamp.timestamp_millis() + interval_ms)
                .unwrap_or(end_ms);
            candles.extend(batch);
        }

        candles.sort_by_key(|c| c.timestamp);
        candles.dedup_by_key(|c| c.timestamp);
        Ok(candles)
    }
}

fn parse_entry(entry: &[String], symbol: &str, interval: Interval) -> Option<Candle> {
    if entry.len() < 6 {
        return None;
    }
    let ts = entry.first()?.parse::<i64>().ok()?;
    let timestamp = DateTime::<Utc>::from_timestamp_millis(ts)?;
    let open = entry.get(1)?.parse::<Decimal>().ok()?;
    let high = entry.get(2)?.parse::<Decimal>().ok()?;
    let low = entry.get(3)?.parse::<Decimal>().ok()?;
    let close = entry.get(4)?.parse::<Decimal>().ok()?;
    let volume = entry.get(5)?.parse::<Decimal>().ok()?;
    Some(Candle {
        symbol: Symbol::from(symbol),
        interval,
        open,
        high,
        low,
        close,
        volume,
        timestamp,
    })
}

#[derive(Debug, Deserialize)]
struct BybitKlineResponse {
    #[serde(rename = "retCode")]
    ret_code: i64,
    #[serde(rename = "retMsg")]
    ret_msg: String,
    result: Option<KlineResult>,
}

#[derive(Debug, Deserialize)]
struct KlineResult {
    list: Vec<Vec<String>>,
}

fn truncate(body: &str, max: usize) -> String {
    if body.len() <= max {
        body.to_string()
    } else {
        format!("{}…", &body[..max])
    }
}
