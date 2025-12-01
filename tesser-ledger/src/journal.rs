use rust_decimal::Decimal;
use serde_json::json;
use tesser_core::{AssetId, Fill, Instrument, InstrumentKind, Side};

use crate::{LedgerEntry, LedgerType};

/// Context required to derive ledger entries from a trade fill.
pub struct FillLedgerContext<'a> {
    pub fill: &'a Fill,
    pub instrument: &'a Instrument,
    pub realized_pnl: Decimal,
}

impl<'a> FillLedgerContext<'a> {
    pub fn new(fill: &'a Fill, instrument: &'a Instrument, realized_pnl: Decimal) -> Self {
        Self {
            fill,
            instrument,
            realized_pnl,
        }
    }
}

/// Build the ledger entries representing cash movements for the provided fill.
pub fn entries_from_fill(ctx: FillLedgerContext<'_>) -> Vec<LedgerEntry> {
    let mut entries = match ctx.instrument.kind {
        InstrumentKind::Spot => spot_entries(ctx.fill, ctx.instrument),
        InstrumentKind::LinearPerpetual | InstrumentKind::InversePerpetual => {
            derivative_entries(ctx.fill, ctx.instrument)
        }
    };
    if !ctx.realized_pnl.is_zero() {
        entries.push(build_entry(
            ctx.instrument.settlement_currency,
            ctx.realized_pnl,
            ctx.fill,
            LedgerType::TradeRealizedPnl,
            Some("realized_pnl"),
        ));
    }
    if let Some(fee) = ctx.fill.fee {
        let fee_asset = ctx.fill.fee_asset.unwrap_or(match ctx.instrument.kind {
            InstrumentKind::Spot => ctx.instrument.quote,
            _ => ctx.instrument.settlement_currency,
        });
        if !fee.is_zero() {
            entries.push(build_entry(
                fee_asset,
                -fee,
                ctx.fill,
                LedgerType::Fee,
                Some("fee"),
            ));
        }
    }
    entries
}

fn spot_entries(fill: &Fill, instrument: &Instrument) -> Vec<LedgerEntry> {
    let qty = fill.fill_quantity;
    let notional = fill.fill_price * qty;
    let mut entries = Vec::new();
    let base_delta = match fill.side {
        Side::Buy => qty,
        Side::Sell => -qty,
    };
    if !base_delta.is_zero() {
        entries.push(build_entry(
            instrument.base,
            base_delta,
            fill,
            LedgerType::Adjustment,
            Some("base"),
        ));
    }
    let quote_delta = match fill.side {
        Side::Buy => -notional,
        Side::Sell => notional,
    };
    if !quote_delta.is_zero() {
        entries.push(build_entry(
            instrument.quote,
            quote_delta,
            fill,
            LedgerType::Adjustment,
            Some("quote"),
        ));
    }
    entries
}

fn derivative_entries(fill: &Fill, instrument: &Instrument) -> Vec<LedgerEntry> {
    let notional = fill.fill_price * fill.fill_quantity;
    let direction = Decimal::from(fill.side.as_i8());
    let settlement_delta = -(notional * direction);
    let mut entries = Vec::new();
    if !settlement_delta.is_zero() {
        entries.push(build_entry(
            instrument.settlement_currency,
            settlement_delta,
            fill,
            LedgerType::Adjustment,
            Some("settlement"),
        ));
    }
    entries
}

fn build_entry(
    asset: AssetId,
    amount: Decimal,
    fill: &Fill,
    entry_type: LedgerType,
    component: Option<&str>,
) -> LedgerEntry {
    let mut entry = LedgerEntry::new(
        asset.exchange,
        asset,
        amount,
        entry_type,
        fill.order_id.to_string(),
    );
    if let Some(kind) = component {
        entry.meta = Some(json!({
            "symbol": fill.symbol.to_string(),
            "component": kind,
        }));
    }
    entry.timestamp = fill.timestamp;
    entry
}
