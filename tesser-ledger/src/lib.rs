//! Ledger primitives and storage backends used by the Tesser runtime.

mod entry;
mod error;
mod journal;
mod parquet;
mod query;
mod repository;
mod sequencer;
mod sqlite;

pub use entry::{LedgerEntry, LedgerType};
pub use error::{LedgerError, LedgerResult};
pub use journal::{entries_from_fill, FillLedgerContext};
pub use parquet::ParquetLedgerRepository;
pub use query::LedgerQuery;
pub use repository::LedgerRepository;
pub use sequencer::LedgerSequencer;
pub use sqlite::SqliteLedgerRepository;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use tesser_core::{AssetId, ExchangeId};

    fn sample_entry(kind: LedgerType, amount: Decimal, seq: u64) -> LedgerEntry {
        LedgerEntry {
            id: uuid::Uuid::new_v4(),
            sequence: seq,
            timestamp: Utc::now(),
            exchange: ExchangeId::from("paper"),
            asset: AssetId::from("paper:USDT"),
            amount,
            entry_type: kind,
            reference_id: format!("ref-{seq}"),
            meta: None,
        }
    }

    #[test]
    fn satisfies_accounting_identity() {
        let entries = vec![
            sample_entry(LedgerType::TransferIn, dec!(100), 1),
            sample_entry(LedgerType::TransferOut, dec!(-25), 2),
            sample_entry(LedgerType::TradeRealizedPnl, dec!(60), 3),
            sample_entry(LedgerType::Fee, dec!(-15), 4),
        ];
        let (assets, liabilities, equity) = summarize(&entries);
        assert_eq!(assets, liabilities + equity);
    }

    fn summarize(entries: &[LedgerEntry]) -> (Decimal, Decimal, Decimal) {
        let mut assets = Decimal::ZERO;
        let mut liabilities = Decimal::ZERO;
        let mut equity = Decimal::ZERO;
        for entry in entries {
            match entry.entry_type {
                LedgerType::TransferIn | LedgerType::TransferOut => assets += entry.amount,
                LedgerType::Fee => liabilities += -entry.amount,
                LedgerType::Funding | LedgerType::TradeRealizedPnl | LedgerType::Adjustment => {
                    equity += entry.amount
                }
            }
        }
        (assets, liabilities, equity)
    }
}
