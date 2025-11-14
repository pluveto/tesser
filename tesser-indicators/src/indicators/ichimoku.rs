//! Ichimoku Cloud indicator implementation.

use std::collections::VecDeque;

use rust_decimal::Decimal;
use tesser_core::Candle;

use crate::core::{Indicator, IndicatorError};

#[derive(Clone, Copy, Debug, PartialEq)]
/// Snapshot of the Ichimoku Cloud state.
pub struct IchimokuOutput {
    /// Tenkan-sen line value.
    pub conversion_line: Decimal,
    /// Kijun-sen line value.
    pub base_line: Decimal,
    /// Senkou Span A projection.
    pub span_a: Decimal,
    /// Senkou Span B projection.
    pub span_b: Decimal,
}

/// Ichimoku Cloud indicator implementation.
pub struct Ichimoku {
    conversion_period: usize,
    base_period: usize,
    span_b_period: usize,
    highs_conv: VecDeque<Decimal>,
    lows_conv: VecDeque<Decimal>,
    highs_base: VecDeque<Decimal>,
    lows_base: VecDeque<Decimal>,
    highs_span_b: VecDeque<Decimal>,
    lows_span_b: VecDeque<Decimal>,
}

impl Ichimoku {
    /// Build a new Ichimoku indicator with custom periods.
    pub fn new(
        conversion_period: usize,
        base_period: usize,
        span_b_period: usize,
    ) -> Result<Self, IndicatorError> {
        if conversion_period == 0 {
            return Err(IndicatorError::invalid_period(
                "Ichimoku",
                conversion_period,
            ));
        }
        if base_period == 0 {
            return Err(IndicatorError::invalid_period("Ichimoku", base_period));
        }
        if span_b_period == 0 {
            return Err(IndicatorError::invalid_period("Ichimoku", span_b_period));
        }
        Ok(Self {
            conversion_period,
            base_period,
            span_b_period,
            highs_conv: VecDeque::with_capacity(conversion_period),
            lows_conv: VecDeque::with_capacity(conversion_period),
            highs_base: VecDeque::with_capacity(base_period),
            lows_base: VecDeque::with_capacity(base_period),
            highs_span_b: VecDeque::with_capacity(span_b_period),
            lows_span_b: VecDeque::with_capacity(span_b_period),
        })
    }

    fn midpoint(highs: &VecDeque<Decimal>, lows: &VecDeque<Decimal>) -> Option<Decimal> {
        let max_high = highs.iter().copied().reduce(Decimal::max)?;
        let min_low = lows.iter().copied().reduce(Decimal::min)?;
        Some((max_high + min_low) / Decimal::from(2))
    }
}

impl Indicator for Ichimoku {
    type Input = Candle;
    type Output = IchimokuOutput;

    fn next(&mut self, input: Self::Input) -> Option<Self::Output> {
        push_queue(&mut self.highs_conv, self.conversion_period, input.high);
        push_queue(&mut self.lows_conv, self.conversion_period, input.low);
        push_queue(&mut self.highs_base, self.base_period, input.high);
        push_queue(&mut self.lows_base, self.base_period, input.low);
        push_queue(&mut self.highs_span_b, self.span_b_period, input.high);
        push_queue(&mut self.lows_span_b, self.span_b_period, input.low);

        if self.highs_span_b.len() < self.span_b_period {
            return None;
        }

        let conversion = Self::midpoint(&self.highs_conv, &self.lows_conv)?;
        let base = Self::midpoint(&self.highs_base, &self.lows_base)?;
        let span_b = Self::midpoint(&self.highs_span_b, &self.lows_span_b)?;
        let span_a = (conversion + base) / Decimal::from(2);

        Some(IchimokuOutput {
            conversion_line: conversion,
            base_line: base,
            span_a,
            span_b,
        })
    }

    fn reset(&mut self) {
        self.highs_conv.clear();
        self.lows_conv.clear();
        self.highs_base.clear();
        self.lows_base.clear();
        self.highs_span_b.clear();
        self.lows_span_b.clear();
    }
}

fn push_queue(queue: &mut VecDeque<Decimal>, period: usize, value: Decimal) {
    queue.push_back(value);
    if queue.len() > period {
        queue.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tesser_core::{Interval, Symbol};

    fn candle(value: f64) -> Candle {
        Candle {
            symbol: Symbol::from("BTCUSDT"),
            interval: Interval::OneMinute,
            open: Decimal::from_f64_retain(value).unwrap(),
            high: Decimal::from_f64_retain(value + 1.0).unwrap(),
            low: Decimal::from_f64_retain(value - 1.0).unwrap(),
            close: Decimal::from_f64_retain(value).unwrap(),
            volume: Decimal::ONE,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn ichimoku_emits_after_warmup() {
        let mut ichi = Ichimoku::new(2, 4, 4).unwrap();
        for val in [1.0, 2.0, 3.0] {
            assert!(ichi.next(candle(val)).is_none());
        }
        assert!(ichi.next(candle(4.0)).is_some());
    }
}
