use crate::proto;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;
use tesser_core::{
    Candle, Fill, Interval, OrderBook, OrderBookLevel, Position, Side, Signal, SignalKind, Tick,
};
use tesser_strategy::StrategyContext;

// --- Helpers ---

pub fn to_decimal_proto(d: Decimal) -> proto::Decimal {
    proto::Decimal {
        value: d.to_string(),
    }
}

pub fn from_decimal_proto(d: proto::Decimal) -> Decimal {
    Decimal::from_str(&d.value).unwrap_or(Decimal::ZERO)
}

pub fn to_timestamp_proto(dt: DateTime<Utc>) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

// --- Enums ---

fn side_to_proto(s: Side) -> proto::Side {
    match s {
        Side::Buy => proto::Side::Buy,
        Side::Sell => proto::Side::Sell,
    }
}

fn interval_to_proto(i: Interval) -> proto::Interval {
    match i {
        Interval::OneSecond => proto::Interval::Interval1s,
        Interval::OneMinute => proto::Interval::Interval1m,
        Interval::FiveMinutes => proto::Interval::Interval5m,
        Interval::FifteenMinutes => proto::Interval::Interval15m,
        Interval::OneHour => proto::Interval::Interval1h,
        Interval::FourHours => proto::Interval::Interval4h,
        Interval::OneDay => proto::Interval::Interval1d,
    }
}

// --- Structs to Proto ---

impl From<Tick> for proto::Tick {
    fn from(t: Tick) -> Self {
        Self {
            symbol: t.symbol,
            price: Some(to_decimal_proto(t.price)),
            size: Some(to_decimal_proto(t.size)),
            side: side_to_proto(t.side) as i32,
            exchange_timestamp: Some(to_timestamp_proto(t.exchange_timestamp)),
            received_at: Some(to_timestamp_proto(t.received_at)),
        }
    }
}

impl From<Candle> for proto::Candle {
    fn from(c: Candle) -> Self {
        Self {
            symbol: c.symbol,
            interval: interval_to_proto(c.interval) as i32,
            open: Some(to_decimal_proto(c.open)),
            high: Some(to_decimal_proto(c.high)),
            low: Some(to_decimal_proto(c.low)),
            close: Some(to_decimal_proto(c.close)),
            volume: Some(to_decimal_proto(c.volume)),
            timestamp: Some(to_timestamp_proto(c.timestamp)),
        }
    }
}

impl From<OrderBook> for proto::OrderBook {
    fn from(b: OrderBook) -> Self {
        Self {
            symbol: b.symbol,
            bids: b.bids.into_iter().map(Into::into).collect(),
            asks: b.asks.into_iter().map(Into::into).collect(),
            timestamp: Some(to_timestamp_proto(b.timestamp)),
        }
    }
}

impl From<OrderBookLevel> for proto::OrderBookLevel {
    fn from(l: OrderBookLevel) -> Self {
        Self {
            price: Some(to_decimal_proto(l.price)),
            size: Some(to_decimal_proto(l.size)),
        }
    }
}

impl From<Fill> for proto::Fill {
    fn from(f: Fill) -> Self {
        Self {
            order_id: f.order_id,
            symbol: f.symbol,
            side: side_to_proto(f.side) as i32,
            fill_price: Some(to_decimal_proto(f.fill_price)),
            fill_quantity: Some(to_decimal_proto(f.fill_quantity)),
            fee: Some(
                f.fee
                    .map(to_decimal_proto)
                    .unwrap_or_else(|| to_decimal_proto(Decimal::ZERO)),
            ),
            timestamp: Some(to_timestamp_proto(f.timestamp)),
        }
    }
}

impl From<Position> for proto::Position {
    fn from(p: Position) -> Self {
        Self {
            symbol: p.symbol,
            side: match p.side {
                Some(Side::Buy) => proto::Side::Buy as i32,
                Some(Side::Sell) => proto::Side::Sell as i32,
                None => proto::Side::Unspecified as i32,
            },
            quantity: Some(to_decimal_proto(p.quantity)),
            entry_price: Some(
                p.entry_price
                    .map(to_decimal_proto)
                    .unwrap_or_else(|| to_decimal_proto(Decimal::ZERO)),
            ),
            unrealized_pnl: Some(to_decimal_proto(p.unrealized_pnl)),
            updated_at: Some(to_timestamp_proto(p.updated_at)),
        }
    }
}

impl<'a> From<&'a StrategyContext> for proto::StrategyContext {
    fn from(ctx: &'a StrategyContext) -> Self {
        Self {
            positions: ctx.positions().iter().cloned().map(Into::into).collect(),
        }
    }
}

// --- Proto to Structs ---

impl From<proto::Signal> for Signal {
    fn from(p: proto::Signal) -> Self {
        let kind = match proto::signal::Kind::try_from(p.kind)
            .unwrap_or(proto::signal::Kind::Unspecified)
        {
            proto::signal::Kind::EnterLong => SignalKind::EnterLong,
            proto::signal::Kind::ExitLong => SignalKind::ExitLong,
            proto::signal::Kind::EnterShort => SignalKind::EnterShort,
            proto::signal::Kind::ExitShort => SignalKind::ExitShort,
            proto::signal::Kind::Flatten => SignalKind::Flatten,
            _ => SignalKind::EnterLong, // Default fallback
        };

        let mut signal = Signal::new(p.symbol, kind, p.confidence);

        if let Some(sl) = p.stop_loss {
            signal.stop_loss = Some(from_decimal_proto(sl));
        }
        if let Some(tp) = p.take_profit {
            signal.take_profit = Some(from_decimal_proto(tp));
        }
        if let Some(note) = if p.note.is_empty() {
            None
        } else {
            Some(p.note)
        } {
            signal.note = Some(note);
        }

        // TODO: Future expansion for execution hints

        signal
    }
}
