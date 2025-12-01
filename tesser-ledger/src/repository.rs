use crate::{LedgerEntry, LedgerQuery, LedgerResult};

/// Abstraction over durable ledger storage engines.
pub trait LedgerRepository: Send + Sync {
    /// Persist a single entry.
    fn append(&self, entry: &LedgerEntry) -> LedgerResult<()> {
        self.append_batch(std::slice::from_ref(entry))
    }

    /// Persist a group of entries atomically.
    fn append_batch(&self, entries: &[LedgerEntry]) -> LedgerResult<()>;

    /// Read the latest persisted sequence value.
    fn latest_sequence(&self) -> LedgerResult<Option<u64>>;

    /// Stream entries matching the supplied query.
    fn query(&self, query: LedgerQuery) -> LedgerResult<Vec<LedgerEntry>>;
}
