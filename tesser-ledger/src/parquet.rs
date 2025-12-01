use std::collections::BTreeMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, Decimal128Array, Decimal128Builder, StringArray, StringBuilder,
    TimestampNanosecondArray, TimestampNanosecondBuilder, UInt64Array, UInt64Builder,
};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef, TimeUnit};
use arrow::record_batch::RecordBatch;
use chrono::{DateTime, Datelike, Utc};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use rust_decimal::Decimal;
use tesser_core::{AssetId, ExchangeId};
use uuid::Uuid;

use crate::{LedgerEntry, LedgerError, LedgerQuery, LedgerRepository, LedgerResult, LedgerType};

const LEDGER_DECIMAL_SCALE: u32 = 18;
const LEDGER_DECIMAL_SCALE_I8: i8 = 18;
const LEDGER_DECIMAL_PRECISION: u8 = 38;

/// File-system backed ledger sink used for analytics and archival workloads.
#[derive(Clone, Debug)]
pub struct ParquetLedgerRepository {
    root: PathBuf,
    schema: SchemaRef,
}

impl ParquetLedgerRepository {
    pub fn new(root: impl Into<PathBuf>) -> LedgerResult<Self> {
        let root = root.into();
        fs::create_dir_all(&root)?;
        Ok(Self {
            root,
            schema: ledger_schema(),
        })
    }

    fn partition_dir(&self, timestamp: DateTime<Utc>) -> PathBuf {
        self.root
            .join(format!("{:04}", timestamp.year()))
            .join(format!("{:02}", timestamp.month()))
            .join(format!("{:02}", timestamp.day()))
    }

    fn write_partition(&self, entries: &[LedgerEntry]) -> LedgerResult<PathBuf> {
        if entries.is_empty() {
            return Err(LedgerError::InvalidState(
                "attempted to write empty ledger partition".into(),
            ));
        }
        let dir = self.partition_dir(entries[0].timestamp);
        fs::create_dir_all(&dir)?;
        let file_name = format!(
            "ledger-{}-{}.parquet",
            entries[0].timestamp.timestamp(),
            Uuid::new_v4()
        );
        let path = dir.join(file_name);
        let file = File::create(&path)?;
        let mut writer = ArrowWriter::try_new(file, self.schema.clone(), None)?;
        let batch = entries_to_batch(entries, &self.schema)?;
        writer.write(&batch)?;
        writer.close()?;
        Ok(path)
    }

    fn list_parquet_files(&self) -> LedgerResult<Vec<PathBuf>> {
        let mut files = Vec::new();
        if !self.root.exists() {
            return Ok(files);
        }
        for year_dir in fs::read_dir(&self.root)? {
            let year_dir = year_dir?;
            if !year_dir.path().is_dir() {
                continue;
            }
            for month_dir in fs::read_dir(year_dir.path())? {
                let month_dir = month_dir?;
                if !month_dir.path().is_dir() {
                    continue;
                }
                for day_dir in fs::read_dir(month_dir.path())? {
                    let day_dir = day_dir?;
                    if day_dir.path().is_dir() {
                        for file in fs::read_dir(day_dir.path())? {
                            let file = file?;
                            if file.path().extension().and_then(|ext| ext.to_str())
                                == Some("parquet")
                            {
                                files.push(file.path());
                            }
                        }
                    } else if day_dir.path().extension().and_then(|ext| ext.to_str())
                        == Some("parquet")
                    {
                        files.push(day_dir.path());
                    }
                }
            }
        }
        Ok(files)
    }

    fn read_file_entries(&self, path: &Path) -> LedgerResult<Vec<LedgerEntry>> {
        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let reader = builder.build()?;
        let mut entries = Vec::new();
        for batch in reader {
            let batch = batch?;
            entries.extend(batch_to_entries(&batch)?);
        }
        Ok(entries)
    }
}

impl LedgerRepository for ParquetLedgerRepository {
    fn append_batch(&self, entries: &[LedgerEntry]) -> LedgerResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut buckets: BTreeMap<(i32, u32, u32), Vec<LedgerEntry>> = BTreeMap::new();
        for entry in entries {
            let date = entry.timestamp.date_naive();
            buckets
                .entry((date.year(), date.month(), date.day()))
                .or_default()
                .push(entry.clone());
        }
        for bucket in buckets.values() {
            self.write_partition(bucket)?;
        }
        Ok(())
    }

    fn latest_sequence(&self) -> LedgerResult<Option<u64>> {
        let mut max_seq = None;
        for path in self.list_parquet_files()? {
            let file_entries = self.read_file_entries(&path)?;
            for entry in file_entries {
                if max_seq.is_none_or(|current| entry.sequence > current) {
                    max_seq = Some(entry.sequence);
                }
            }
        }
        Ok(max_seq)
    }

    fn query(&self, query: LedgerQuery) -> LedgerResult<Vec<LedgerEntry>> {
        let mut rows = Vec::new();
        for path in self.list_parquet_files()? {
            rows.extend(self.read_file_entries(&path)?);
        }
        rows.retain(|entry| matches_query(entry, &query));
        rows.sort_by_key(|entry| entry.sequence);
        if !query.ascending {
            rows.reverse();
        }
        if let Some(limit) = query.limit {
            rows.truncate(limit);
        }
        Ok(rows)
    }
}

fn matches_query(entry: &LedgerEntry, query: &LedgerQuery) -> bool {
    if let Some(exchange) = query.exchange {
        if entry.exchange != exchange {
            return false;
        }
    }
    if let Some(asset) = query.asset {
        if entry.asset != asset {
            return false;
        }
    }
    if let Some(entry_type) = query.entry_type {
        if entry.entry_type != entry_type {
            return false;
        }
    }
    if let Some(start) = query.start_sequence {
        if entry.sequence < start {
            return false;
        }
    }
    if let Some(end) = query.end_sequence {
        if entry.sequence > end {
            return false;
        }
    }
    if let Some(start) = query.start_time {
        if entry.timestamp < start {
            return false;
        }
    }
    if let Some(end) = query.end_time {
        if entry.timestamp > end {
            return false;
        }
    }
    true
}

fn ledger_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("sequence", DataType::UInt64, false),
        Field::new("id", DataType::Utf8, false),
        Field::new(
            "timestamp",
            DataType::Timestamp(TimeUnit::Nanosecond, None),
            false,
        ),
        Field::new("exchange", DataType::Utf8, false),
        Field::new("asset", DataType::Utf8, false),
        Field::new(
            "amount",
            DataType::Decimal128(LEDGER_DECIMAL_PRECISION, LEDGER_DECIMAL_SCALE_I8),
            false,
        ),
        Field::new("entry_type", DataType::Utf8, false),
        Field::new("reference_id", DataType::Utf8, false),
        Field::new("meta", DataType::Utf8, true),
    ]))
}

fn entries_to_batch(entries: &[LedgerEntry], schema: &SchemaRef) -> LedgerResult<RecordBatch> {
    let mut sequences = UInt64Builder::new();
    let mut ids = StringBuilder::new();
    let mut timestamps = TimestampNanosecondBuilder::new();
    let mut exchanges = StringBuilder::new();
    let mut assets = StringBuilder::new();
    let mut amounts = Decimal128Builder::new().with_data_type(DataType::Decimal128(
        LEDGER_DECIMAL_PRECISION,
        LEDGER_DECIMAL_SCALE_I8,
    ));
    let mut types = StringBuilder::new();
    let mut references = StringBuilder::new();
    let mut metas = StringBuilder::new();

    for entry in entries {
        sequences.append_value(entry.sequence);
        ids.append_value(entry.id.to_string());
        if let Some(ts) = entry.timestamp.timestamp_nanos_opt() {
            timestamps.append_value(ts);
        } else {
            return Err(LedgerError::Serialization(
                "timestamp precision exceeds nanoseconds".into(),
            ));
        }
        exchanges.append_value(entry.exchange);
        assets.append_value(entry.asset);
        let encoded = decimal_to_i128(entry.amount)?;
        amounts.append_value(encoded);
        types.append_value(entry.entry_type.as_str());
        references.append_value(&entry.reference_id);
        if let Some(meta) = &entry.meta {
            metas.append_value(meta.to_string());
        } else {
            metas.append_null();
        }
    }

    let columns: Vec<ArrayRef> = vec![
        Arc::new(sequences.finish()),
        Arc::new(ids.finish()),
        Arc::new(timestamps.finish()),
        Arc::new(exchanges.finish()),
        Arc::new(assets.finish()),
        Arc::new(amounts.finish()),
        Arc::new(types.finish()),
        Arc::new(references.finish()),
        Arc::new(metas.finish()),
    ];

    RecordBatch::try_new(schema.clone(), columns).map_err(Into::into)
}

fn batch_to_entries(batch: &RecordBatch) -> LedgerResult<Vec<LedgerEntry>> {
    let sequences = batch
        .column_by_name("sequence")
        .and_then(|array| array.as_any().downcast_ref::<UInt64Array>())
        .ok_or_else(|| {
            LedgerError::InvalidState("missing sequence column in ledger parquet".into())
        })?;
    let ids = batch
        .column_by_name("id")
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| LedgerError::InvalidState("missing id column in ledger parquet".into()))?;
    let timestamps = batch
        .column_by_name("timestamp")
        .and_then(|array| array.as_any().downcast_ref::<TimestampNanosecondArray>())
        .ok_or_else(|| {
            LedgerError::InvalidState("missing timestamp column in ledger parquet".into())
        })?;
    let exchanges = batch
        .column_by_name("exchange")
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            LedgerError::InvalidState("missing exchange column in ledger parquet".into())
        })?;
    let assets = batch
        .column_by_name("asset")
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            LedgerError::InvalidState("missing asset column in ledger parquet".into())
        })?;
    let amounts = batch
        .column_by_name("amount")
        .and_then(|array| array.as_any().downcast_ref::<Decimal128Array>())
        .ok_or_else(|| {
            LedgerError::InvalidState("missing amount column in ledger parquet".into())
        })?;
    let types = batch
        .column_by_name("entry_type")
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            LedgerError::InvalidState("missing entry_type column in ledger parquet".into())
        })?;
    let references = batch
        .column_by_name("reference_id")
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            LedgerError::InvalidState("missing reference_id column in ledger parquet".into())
        })?;
    let metas = batch
        .column_by_name("meta")
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| LedgerError::InvalidState("missing meta column in ledger parquet".into()))?;

    let mut entries = Vec::with_capacity(batch.num_rows());
    for idx in 0..batch.num_rows() {
        let ts_value = timestamps.value(idx);
        let secs = ts_value / 1_000_000_000;
        let nanos = (ts_value % 1_000_000_000) as u32;
        let timestamp = DateTime::<Utc>::from_timestamp(secs, nanos).ok_or_else(|| {
            LedgerError::Serialization(format!("invalid timestamp components: {secs} {nanos}"))
        })?;
        let exchange = ExchangeId::from_str(exchanges.value(idx)).map_err(|err| {
            LedgerError::Serialization(format!("invalid exchange {}: {err}", exchanges.value(idx)))
        })?;
        let asset = AssetId::from_str(assets.value(idx)).map_err(|err| {
            LedgerError::Serialization(format!("invalid asset {}: {err}", assets.value(idx)))
        })?;
        let amount = decimal_from_i128(amounts.value(idx))?;
        let entry_type =
            LedgerType::from_str(types.value(idx)).map_err(LedgerError::Serialization)?;
        let meta = if metas.is_null(idx) {
            None
        } else {
            Some(
                serde_json::from_str::<serde_json::Value>(metas.value(idx)).map_err(|err| {
                    LedgerError::Serialization(format!("invalid ledger meta payload: {err}"))
                })?,
            )
        };

        let entry_id = Uuid::parse_str(ids.value(idx)).map_err(|err| {
            LedgerError::Serialization(format!("invalid ledger id {}: {err}", ids.value(idx)))
        })?;

        entries.push(LedgerEntry {
            id: entry_id,
            sequence: sequences.value(idx),
            timestamp,
            exchange,
            asset,
            amount,
            entry_type,
            reference_id: references.value(idx).to_string(),
            meta,
        });
    }
    Ok(entries)
}

fn decimal_to_i128(value: Decimal) -> LedgerResult<i128> {
    let mut normalized = value;
    let scale = normalized.scale();
    if scale > LEDGER_DECIMAL_SCALE {
        normalized = normalized.round_dp(LEDGER_DECIMAL_SCALE);
    }
    let diff = LEDGER_DECIMAL_SCALE.saturating_sub(normalized.scale());
    let factor = 10i128
        .checked_pow(diff)
        .ok_or_else(|| LedgerError::Serialization("decimal scaling overflow".into()))?;
    normalized
        .mantissa()
        .checked_mul(factor)
        .ok_or_else(|| LedgerError::Serialization("decimal mantissa overflow".into()))
}

fn decimal_from_i128(value: i128) -> LedgerResult<Decimal> {
    Ok(Decimal::from_i128_with_scale(value, LEDGER_DECIMAL_SCALE))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use tempfile::tempdir;

    #[test]
    fn parquet_roundtrip() {
        let dir = tempdir().unwrap();
        let repo = ParquetLedgerRepository::new(dir.path()).unwrap();
        let mut entries = Vec::new();
        for seq in 1..=5 {
            entries.push(LedgerEntry {
                id: Uuid::new_v4(),
                sequence: seq,
                timestamp: Utc::now(),
                exchange: ExchangeId::from("paper"),
                asset: AssetId::from("paper:USDT"),
                amount: dec!(1.25) * Decimal::from(seq as i64),
                entry_type: LedgerType::TransferIn,
                reference_id: format!("ref-{seq}"),
                meta: None,
            });
        }
        repo.append_batch(&entries).unwrap();
        let loaded = repo.query(LedgerQuery::default()).unwrap();
        assert_eq!(loaded.len(), 5);
    }
}
