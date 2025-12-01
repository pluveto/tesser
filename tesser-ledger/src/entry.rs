use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use tesser_core::{AssetId, ExchangeId};
use uuid::Uuid;

/// Canonical ledger record describing a single balance delta.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub sequence: u64,
    pub timestamp: DateTime<Utc>,
    pub exchange: ExchangeId,
    pub asset: AssetId,
    pub amount: Decimal,
    pub entry_type: LedgerType,
    pub reference_id: String,
    pub meta: Option<serde_json::Value>,
}

impl LedgerEntry {
    /// Creates a new ledger entry with a zero sequence number.
    pub fn new(
        exchange: ExchangeId,
        asset: AssetId,
        amount: Decimal,
        entry_type: LedgerType,
        reference_id: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            sequence: 0,
            timestamp: Utc::now(),
            exchange,
            asset,
            amount,
            entry_type,
            reference_id: reference_id.into(),
            meta: None,
        }
    }

    /// Assign the monotonic sequence number used for replay.
    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.sequence = sequence;
        self
    }
}

/// Enumerates the supported ledger line item categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerType {
    TradeRealizedPnl,
    Fee,
    Funding,
    TransferIn,
    TransferOut,
    Adjustment,
}

impl LedgerType {
    pub fn as_str(self) -> &'static str {
        match self {
            LedgerType::TradeRealizedPnl => "trade_realized_pnl",
            LedgerType::Fee => "fee",
            LedgerType::Funding => "funding",
            LedgerType::TransferIn => "transfer_in",
            LedgerType::TransferOut => "transfer_out",
            LedgerType::Adjustment => "adjustment",
        }
    }
}

impl fmt::Display for LedgerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LedgerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trade_realized_pnl" => Ok(LedgerType::TradeRealizedPnl),
            "fee" => Ok(LedgerType::Fee),
            "funding" => Ok(LedgerType::Funding),
            "transfer_in" => Ok(LedgerType::TransferIn),
            "transfer_out" => Ok(LedgerType::TransferOut),
            "adjustment" => Ok(LedgerType::Adjustment),
            other => Err(format!("unknown ledger type: {other}")),
        }
    }
}
