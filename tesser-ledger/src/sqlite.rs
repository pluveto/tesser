use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use rust_decimal::Decimal;
use tesser_core::{AssetId, ExchangeId};
use uuid::Uuid;

use crate::{LedgerEntry, LedgerError, LedgerQuery, LedgerRepository, LedgerResult, LedgerType};

const LEDGER_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS ledger_entries (
    sequence INTEGER PRIMARY KEY,
    entry_id TEXT NOT NULL UNIQUE,
    timestamp TEXT NOT NULL,
    exchange TEXT NOT NULL,
    asset TEXT NOT NULL,
    amount TEXT NOT NULL,
    entry_type TEXT NOT NULL,
    reference_id TEXT NOT NULL,
    meta TEXT
);
CREATE INDEX IF NOT EXISTS ledger_idx_timestamp_exchange_asset
    ON ledger_entries(timestamp, exchange, asset);
CREATE INDEX IF NOT EXISTS ledger_idx_reference
    ON ledger_entries(reference_id);
"#;

/// SQLite-backed ledger repository used by the live runtime.
#[derive(Clone, Debug)]
pub struct SqliteLedgerRepository {
    path: PathBuf,
}

impl SqliteLedgerRepository {
    pub fn new(path: impl Into<PathBuf>) -> LedgerResult<Self> {
        let repo = Self { path: path.into() };
        repo.initialize_schema()?;
        Ok(repo)
    }

    fn initialize_schema(&self) -> LedgerResult<()> {
        let conn = self.connect()?;
        conn.execute_batch(LEDGER_SCHEMA)?;
        Ok(())
    }

    fn connect(&self) -> LedgerResult<Connection> {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(&self.path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        Ok(conn)
    }
}

impl LedgerRepository for SqliteLedgerRepository {
    fn append_batch(&self, entries: &[LedgerEntry]) -> LedgerResult<()> {
        if entries.is_empty() {
            return Ok(());
        }
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        for entry in entries {
            tx.execute(
                "INSERT INTO ledger_entries (
                    sequence, entry_id, timestamp, exchange, asset, amount, entry_type, reference_id, meta
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    entry.sequence as i64,
                    entry.id.to_string(),
                    entry.timestamp.to_rfc3339(),
                    entry.exchange.to_string(),
                    entry.asset.to_string(),
                    entry.amount.to_string(),
                    entry.entry_type.as_str(),
                    entry.reference_id,
                    entry.meta.as_ref().map(|value| value.to_string())
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    fn latest_sequence(&self) -> LedgerResult<Option<u64>> {
        let conn = self.connect()?;
        let seq: Option<Option<i64>> = conn
            .query_row("SELECT MAX(sequence) FROM ledger_entries", [], |row| {
                row.get::<_, Option<i64>>(0)
            })
            .optional()?;
        Ok(seq.flatten().map(|value| value as u64))
    }

    fn query(&self, query: LedgerQuery) -> LedgerResult<Vec<LedgerEntry>> {
        let conn = self.connect()?;
        let mut sql = String::from(
            "SELECT sequence, entry_id, timestamp, exchange, asset, amount, entry_type, reference_id, meta
             FROM ledger_entries
             WHERE (?1 IS NULL OR exchange = ?1)
               AND (?2 IS NULL OR asset = ?2)
               AND (?3 IS NULL OR entry_type = ?3)
               AND (?4 IS NULL OR sequence >= ?4)
               AND (?5 IS NULL OR sequence <= ?5)
               AND (?6 IS NULL OR timestamp >= ?6)
               AND (?7 IS NULL OR timestamp <= ?7)"
        );
        sql.push_str(if query.ascending {
            " ORDER BY sequence ASC"
        } else {
            " ORDER BY sequence DESC"
        });
        if query.limit.is_some() {
            sql.push_str(" LIMIT ?8");
        }

        let mut params: Vec<Value> = Vec::with_capacity(8);
        params.push(optional_text(query.exchange.map(|id| id.to_string())));
        params.push(optional_text(query.asset.map(|id| id.to_string())));
        params.push(optional_text(
            query.entry_type.map(|t| t.as_str().to_string()),
        ));
        params.push(optional_int(query.start_sequence));
        params.push(optional_int(query.end_sequence));
        params.push(optional_text(query.start_time.map(|ts| ts.to_rfc3339())));
        params.push(optional_text(query.end_time.map(|ts| ts.to_rfc3339())));
        if let Some(limit) = query.limit {
            params.push(Value::Integer(limit as i64));
        }

        let mut stmt = conn.prepare(&sql)?;
        let mut rows = if params.is_empty() {
            stmt.query([])?
        } else {
            stmt.query(params_from_iter(params.iter()))?
        };
        let mut entries = Vec::new();
        while let Some(row) = rows.next()? {
            entries.push(row_to_entry(row)?);
        }
        Ok(entries)
    }
}

fn optional_text(value: Option<String>) -> Value {
    value.map(Value::from).unwrap_or(Value::Null)
}

fn optional_int(value: Option<u64>) -> Value {
    value
        .map(|v| Value::Integer(v as i64))
        .unwrap_or(Value::Null)
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> LedgerResult<LedgerEntry> {
    let sequence: i64 = row.get(0)?;
    let entry_id: String = row.get(1)?;
    let timestamp_str: String = row.get(2)?;
    let exchange_str: String = row.get(3)?;
    let asset_str: String = row.get(4)?;
    let amount_str: String = row.get(5)?;
    let entry_type_str: String = row.get(6)?;
    let reference_id: String = row.get(7)?;
    let meta_value: Option<String> = row.get(8)?;

    let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
        .map_err(|err| {
            LedgerError::Serialization(format!("invalid timestamp {timestamp_str}: {err}"))
        })?
        .with_timezone(&Utc);
    let exchange = ExchangeId::from_str(&exchange_str).map_err(|err| {
        LedgerError::Serialization(format!("invalid exchange {exchange_str}: {err}"))
    })?;
    let asset = AssetId::from_str(&asset_str)
        .map_err(|err| LedgerError::Serialization(format!("invalid asset {asset_str}: {err}")))?;
    let amount = Decimal::from_str(&amount_str).map_err(|err| {
        LedgerError::Serialization(format!("invalid decimal {amount_str}: {err}"))
    })?;
    let entry_type = LedgerType::from_str(&entry_type_str).map_err(LedgerError::Serialization)?;
    let meta = if let Some(json) = meta_value {
        Some(serde_json::from_str(&json).map_err(|err| {
            LedgerError::Serialization(format!("invalid ledger meta payload: {err}"))
        })?)
    } else {
        None
    };

    Ok(LedgerEntry {
        id: Uuid::parse_str(&entry_id).map_err(|err| {
            LedgerError::Serialization(format!("invalid ledger id {entry_id}: {err}"))
        })?,
        sequence: sequence as u64,
        timestamp,
        exchange,
        asset,
        amount,
        entry_type,
        reference_id,
        meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use tempfile::tempdir;

    #[test]
    fn sqlite_roundtrip() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("ledger.db");
        let repo = SqliteLedgerRepository::new(&db_path).unwrap();
        let mut entry = LedgerEntry::new(
            ExchangeId::from("paper"),
            AssetId::from("paper:USDT"),
            dec!(12.5),
            LedgerType::TransferIn,
            "init",
        );
        entry.sequence = 1;
        repo.append(&entry).unwrap();

        let result = repo
            .query(LedgerQuery::default().with_sequence_range(Some(1), Some(10)))
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].amount, dec!(12.5));
        assert_eq!(result[0].entry_type, LedgerType::TransferIn);
    }
}
