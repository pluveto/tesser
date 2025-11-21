use std::collections::VecDeque;
use std::path::PathBuf;
use std::pin::Pin;
use std::str::FromStr;

use anyhow::{anyhow, Context, Result};
use arrow::array::{Array, Decimal128Array, Int8Array, StringArray, TimestampNanosecondArray};
use arrow::datatypes::SchemaRef;
use arrow::record_batch::RecordBatch;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use parquet::arrow::async_reader::ParquetRecordBatchStream;
use parquet::arrow::ParquetRecordBatchStreamBuilder;
use rust_decimal::Decimal;
use tokio::fs::File;

use tesser_broker::{BrokerError, BrokerInfo, BrokerResult, MarketStream};
use tesser_core::{Candle, Interval, OrderBook, Side, Symbol, Tick};

const DEFAULT_BATCH_SIZE: usize = 4_096;

/// Market stream backed by on-disk parquet files (flight recorder output).
pub struct ParquetMarketStream {
    info: BrokerInfo,
    ticks: Option<TickCursor>,
    candles: Option<CandleCursor>,
}

unsafe impl Sync for ParquetMarketStream {}

impl ParquetMarketStream {
    /// Build a stream configured with tick and candle partitions.
    pub fn new(symbols: Vec<Symbol>, tick_paths: Vec<PathBuf>, candle_paths: Vec<PathBuf>) -> Self {
        let info = BrokerInfo {
            name: "parquet-replay".into(),
            markets: symbols,
            supports_testnet: true,
        };
        Self {
            info,
            ticks: if tick_paths.is_empty() {
                None
            } else {
                Some(TickCursor::new(tick_paths))
            },
            candles: if candle_paths.is_empty() {
                None
            } else {
                Some(CandleCursor::new(candle_paths))
            },
        }
    }

    /// Convenience helper when only candles are being replayed.
    pub fn with_candles(symbols: Vec<Symbol>, candle_paths: Vec<PathBuf>) -> Self {
        Self::new(symbols, Vec::new(), candle_paths)
    }
}

#[async_trait]
impl MarketStream for ParquetMarketStream {
    type Subscription = ();

    fn name(&self) -> &str {
        &self.info.name
    }

    fn info(&self) -> Option<&BrokerInfo> {
        Some(&self.info)
    }

    async fn subscribe(&mut self, _subscription: Self::Subscription) -> BrokerResult<()> {
        Ok(())
    }

    async fn next_tick(&mut self) -> BrokerResult<Option<Tick>> {
        match &mut self.ticks {
            Some(cursor) => cursor.next().await.map_err(map_err),
            None => Ok(None),
        }
    }

    async fn next_candle(&mut self) -> BrokerResult<Option<Candle>> {
        match &mut self.candles {
            Some(cursor) => cursor.next().await.map_err(map_err),
            None => Ok(None),
        }
    }

    async fn next_order_book(&mut self) -> BrokerResult<Option<OrderBook>> {
        Ok(None)
    }
}

fn map_err(err: anyhow::Error) -> BrokerError {
    BrokerError::Other(err.to_string())
}

struct TickCursor {
    loader: BatchLoader,
    columns: Option<TickColumns>,
}

unsafe impl Sync for TickCursor {}

impl TickCursor {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            loader: BatchLoader::new(paths),
            columns: None,
        }
    }

    async fn next(&mut self) -> Result<Option<Tick>> {
        loop {
            if !self.loader.ensure_batch().await? {
                return Ok(None);
            }
            if let Some(schema) = self.loader.take_schema_update() {
                self.columns = Some(TickColumns::from_schema(&schema)?);
            }
            if let Some((batch, row)) = self.loader.next_row() {
                let columns = self
                    .columns
                    .as_ref()
                    .ok_or_else(|| anyhow!("tick schema not initialized"))?;
                return decode_tick(&batch, row, columns).map(Some);
            }
        }
    }
}

struct CandleCursor {
    loader: BatchLoader,
    columns: Option<CandleColumns>,
}

unsafe impl Sync for CandleCursor {}

impl CandleCursor {
    fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            loader: BatchLoader::new(paths),
            columns: None,
        }
    }

    async fn next(&mut self) -> Result<Option<Candle>> {
        loop {
            if !self.loader.ensure_batch().await? {
                return Ok(None);
            }
            if let Some(schema) = self.loader.take_schema_update() {
                self.columns = Some(CandleColumns::from_schema(&schema)?);
            }
            if let Some((batch, row)) = self.loader.next_row() {
                let columns = self
                    .columns
                    .as_ref()
                    .ok_or_else(|| anyhow!("candle schema not initialized"))?;
                return decode_candle(&batch, row, columns).map(Some);
            }
        }
    }
}

struct BatchLoader {
    files: VecDeque<PathBuf>,
    stream: Option<Pin<Box<ParquetRecordBatchStream<File>>>>,
    batch: Option<RecordBatch>,
    row_index: usize,
    schema_update: Option<SchemaRef>,
    batch_size: usize,
}

unsafe impl Sync for BatchLoader {}

impl BatchLoader {
    fn new(mut paths: Vec<PathBuf>) -> Self {
        paths.sort();
        Self {
            files: paths.into(),
            stream: None,
            batch: None,
            row_index: 0,
            schema_update: None,
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    async fn ensure_batch(&mut self) -> Result<bool> {
        loop {
            if let Some(batch) = &self.batch {
                if self.row_index < batch.num_rows() {
                    return Ok(true);
                }
                self.batch = None;
            }

            if let Some(stream) = self.stream.as_mut() {
                match stream.next().await {
                    Some(Ok(batch)) => {
                        self.row_index = 0;
                        self.batch = Some(batch);
                        continue;
                    }
                    Some(Err(err)) => return Err(err.into()),
                    None => {
                        self.stream = None;
                    }
                }
            }

            if !self.open_next_stream().await? {
                return Ok(false);
            }
        }
    }

    fn next_row(&mut self) -> Option<(RecordBatch, usize)> {
        let batch = self.batch.as_ref()?.clone();
        let row = self.row_index;
        self.row_index += 1;
        Some((batch, row))
    }

    fn take_schema_update(&mut self) -> Option<SchemaRef> {
        self.schema_update.take()
    }

    async fn open_next_stream(&mut self) -> Result<bool> {
        let Some(path) = self.files.pop_front() else {
            return Ok(false);
        };
        let file = File::open(&path)
            .await
            .with_context(|| format!("failed to open {}", path.display()))?;
        let mut builder = ParquetRecordBatchStreamBuilder::new(file)
            .await
            .with_context(|| format!("failed to read parquet metadata from {}", path.display()))?;
        builder = builder.with_batch_size(self.batch_size);
        let schema = builder.schema().clone();
        let stream = builder
            .build()
            .with_context(|| format!("failed to build parquet stream for {}", path.display()))?;
        self.stream = Some(Box::pin(stream));
        self.schema_update = Some(schema);
        Ok(true)
    }
}

#[derive(Clone, Copy)]
struct TickColumns {
    symbol: usize,
    price: usize,
    size: usize,
    side: usize,
    exchange_ts: usize,
    received_ts: usize,
}

impl TickColumns {
    fn from_schema(schema: &SchemaRef) -> Result<Self> {
        Ok(Self {
            symbol: column_index(schema, "symbol")?,
            price: column_index(schema, "price")?,
            size: column_index(schema, "size")?,
            side: column_index(schema, "side")?,
            exchange_ts: column_index(schema, "exchange_timestamp")?,
            received_ts: column_index(schema, "received_at")?,
        })
    }
}

#[derive(Clone, Copy)]
struct CandleColumns {
    symbol: usize,
    interval: usize,
    open: usize,
    high: usize,
    low: usize,
    close: usize,
    volume: usize,
    timestamp: usize,
}

impl CandleColumns {
    fn from_schema(schema: &SchemaRef) -> Result<Self> {
        Ok(Self {
            symbol: column_index(schema, "symbol")?,
            interval: column_index(schema, "interval")?,
            open: column_index(schema, "open")?,
            high: column_index(schema, "high")?,
            low: column_index(schema, "low")?,
            close: column_index(schema, "close")?,
            volume: column_index(schema, "volume")?,
            timestamp: column_index(schema, "timestamp")?,
        })
    }
}

fn column_index(schema: &SchemaRef, name: &str) -> Result<usize> {
    schema
        .column_with_name(name)
        .map(|(idx, _)| idx)
        .ok_or_else(|| anyhow!("column '{name}' missing from parquet schema"))
}

fn decode_tick(batch: &RecordBatch, row: usize, columns: &TickColumns) -> Result<Tick> {
    let symbol = string_value(batch, columns.symbol, row)?;
    let price = decimal_value(batch, columns.price, row)?;
    let size = decimal_value(batch, columns.size, row)?;
    let side = side_value(batch, columns.side, row)?;
    let exchange_timestamp = timestamp_value(batch, columns.exchange_ts, row)?;
    let received_at = timestamp_value(batch, columns.received_ts, row)?;
    Ok(Tick {
        symbol,
        price,
        size,
        side,
        exchange_timestamp,
        received_at,
    })
}

fn decode_candle(batch: &RecordBatch, row: usize, columns: &CandleColumns) -> Result<Candle> {
    let symbol = string_value(batch, columns.symbol, row)?;
    let interval_raw = string_value(batch, columns.interval, row)?;
    let interval = Interval::from_str(&interval_raw)
        .map_err(|err| anyhow!("invalid interval '{interval_raw}': {err}"))?;
    let open = decimal_value(batch, columns.open, row)?;
    let high = decimal_value(batch, columns.high, row)?;
    let low = decimal_value(batch, columns.low, row)?;
    let close = decimal_value(batch, columns.close, row)?;
    let volume = decimal_value(batch, columns.volume, row)?;
    let timestamp = timestamp_value(batch, columns.timestamp, row)?;
    Ok(Candle {
        symbol,
        interval,
        open,
        high,
        low,
        close,
        volume,
        timestamp,
    })
}

fn string_value(batch: &RecordBatch, column: usize, row: usize) -> Result<Symbol> {
    let array = as_array::<StringArray>(batch, column)?;
    if array.is_null(row) {
        return Err(anyhow!("column {column} contains null string"));
    }
    Ok(array.value(row).to_string())
}

fn decimal_value(batch: &RecordBatch, column: usize, row: usize) -> Result<Decimal> {
    let array = as_array::<Decimal128Array>(batch, column)?;
    if array.is_null(row) {
        return Err(anyhow!("column {column} contains null decimal"));
    }
    Ok(Decimal::from_i128_with_scale(
        array.value(row),
        array.scale() as u32,
    ))
}

fn timestamp_value(batch: &RecordBatch, column: usize, row: usize) -> Result<DateTime<Utc>> {
    let array = as_array::<TimestampNanosecondArray>(batch, column)?;
    if array.is_null(row) {
        return Err(anyhow!("column {column} contains null timestamp"));
    }
    let nanos = array.value(row);
    let secs = nanos.div_euclid(1_000_000_000);
    let sub = nanos.rem_euclid(1_000_000_000) as u32;
    DateTime::<Utc>::from_timestamp(secs, sub)
        .ok_or_else(|| anyhow!("timestamp overflow for value {nanos}"))
}

fn side_value(batch: &RecordBatch, column: usize, row: usize) -> Result<Side> {
    let array = as_array::<Int8Array>(batch, column)?;
    if array.is_null(row) {
        return Err(anyhow!("column {column} contains null side"));
    }
    Ok(if array.value(row) >= 0 {
        Side::Buy
    } else {
        Side::Sell
    })
}

fn as_array<T: Array + 'static>(batch: &RecordBatch, column: usize) -> Result<&T> {
    batch
        .column(column)
        .as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| anyhow!("column {column} type mismatch"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use parquet::arrow::ArrowWriter;
    use parquet::file::properties::WriterProperties;
    use rust_decimal::Decimal;
    use tempfile::tempdir;
    use tesser_core::{Interval, Side};

    use crate::encoding::{candles_to_batch, ticks_to_batch};

    fn write_parquet_file(path: &PathBuf, batch: &RecordBatch) -> Result<()> {
        let file = std::fs::File::create(path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        let props = WriterProperties::builder().build();
        let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
        writer.write(batch)?;
        writer.close().map(|_| ()).map_err(Into::into)
    }

    fn sample_candles() -> Vec<Candle> {
        vec![Candle {
            symbol: "BTCUSDT".into(),
            interval: Interval::OneMinute,
            open: Decimal::ONE,
            high: Decimal::new(2, 0),
            low: Decimal::ZERO,
            close: Decimal::new(15, 1),
            volume: Decimal::new(5, 0),
            timestamp: Utc::now(),
        }]
    }

    fn sample_ticks() -> Vec<Tick> {
        vec![Tick {
            symbol: "BTCUSDT".into(),
            price: Decimal::new(20_000, 0),
            size: Decimal::new(1, 0),
            side: Side::Buy,
            exchange_timestamp: Utc::now(),
            received_at: Utc::now(),
        }]
    }

    #[tokio::test]
    async fn replays_candles_from_parquet() -> Result<()> {
        let tmp = tempdir()?;
        let path = tmp.path().join("candles.parquet");
        let candles = sample_candles();
        let batch = candles_to_batch(&candles)?;
        write_parquet_file(&path, &batch)?;

        let mut stream = ParquetMarketStream::with_candles(vec!["BTCUSDT".into()], vec![path]);
        let first = stream
            .next_candle()
            .await
            .context("expected candle")?
            .expect("candle available");
        assert_eq!(first.symbol, candles[0].symbol);
        Ok(())
    }

    #[tokio::test]
    async fn replays_ticks_from_parquet() -> Result<()> {
        let tmp = tempdir()?;
        let path = tmp.path().join("ticks.parquet");
        let ticks = sample_ticks();
        let batch = ticks_to_batch(&ticks)?;
        write_parquet_file(&path, &batch)?;

        let mut stream = ParquetMarketStream::new(vec!["BTCUSDT".into()], vec![path], Vec::new());
        let first = stream
            .next_tick()
            .await
            .context("expected tick")?
            .expect("tick available");
        assert_eq!(first.price, ticks[0].price);
        Ok(())
    }
}
