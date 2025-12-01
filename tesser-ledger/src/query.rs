use chrono::{DateTime, Utc};
use tesser_core::{AssetId, ExchangeId};

use crate::LedgerType;

/// Filter describing which ledger entries to load from storage.
#[derive(Clone, Debug, Default)]
pub struct LedgerQuery {
    pub exchange: Option<ExchangeId>,
    pub asset: Option<AssetId>,
    pub entry_type: Option<LedgerType>,
    pub start_sequence: Option<u64>,
    pub end_sequence: Option<u64>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub ascending: bool,
}

impl LedgerQuery {
    pub fn with_exchange(mut self, exchange: ExchangeId) -> Self {
        self.exchange = Some(exchange);
        self
    }

    pub fn with_asset(mut self, asset: AssetId) -> Self {
        self.asset = Some(asset);
        self
    }

    pub fn with_type(mut self, entry_type: LedgerType) -> Self {
        self.entry_type = Some(entry_type);
        self
    }

    pub fn with_sequence_range(mut self, start: Option<u64>, end: Option<u64>) -> Self {
        self.start_sequence = start;
        self.end_sequence = end;
        self
    }

    pub fn with_time_range(
        mut self,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
    ) -> Self {
        self.start_time = start;
        self.end_time = end;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn descending(mut self) -> Self {
        self.ascending = false;
        self
    }
}
