//! Average True Range indicator implementation.

use rust_decimal::Decimal;
use tesser_core::Candle;

use crate::core::{Indicator, IndicatorError};

/// Average True Range indicator.
pub struct Atr {
    period: usize,
    prev_close: Option<Decimal>,
    atr: Option<Decimal>,
    warmup_sum: Decimal,
    warmup_count: usize,
}

impl Atr {
    /// Create a new ATR indicator with the provided period.
    pub fn new(period: usize) -> Result<Self, IndicatorError> {
        if period == 0 {
            return Err(IndicatorError::invalid_period("ATR", period));
        }
        Ok(Self {
            period,
            prev_close: None,
            atr: None,
            warmup_sum: Decimal::ZERO,
            warmup_count: 0,
        })
    }

    fn true_range(&self, candle: &Candle, prev_close: Decimal) -> Decimal {
        let high_low = candle.high - candle.low;
        let high_close = (candle.high - prev_close).abs();
        let low_close = (candle.low - prev_close).abs();
        high_low.max(high_close).max(low_close)
    }
}

impl Indicator for Atr {
    type Input = Candle;
    type Output = Decimal;

    fn next(&mut self, input: Self::Input) -> Option<Self::Output> {
        let prev_close = self.prev_close.unwrap_or(input.close);
        let tr = self.true_range(&input, prev_close);
        self.prev_close = Some(input.close);

        if let Some(current) = self.atr {
            let factor = Decimal::from(self.period as i64 - 1);
            let next = (current * factor + tr) / Decimal::from(self.period as i64);
            self.atr = Some(next);
            Some(next)
        } else {
            self.warmup_sum += tr;
            self.warmup_count += 1;
            if self.warmup_count == self.period {
                let init = self.warmup_sum / Decimal::from(self.period as i64);
                self.atr = Some(init);
                self.warmup_sum = Decimal::ZERO;
                Some(init)
            } else {
                None
            }
        }
    }

    fn reset(&mut self) {
        self.prev_close = None;
        self.atr = None;
        self.warmup_sum = Decimal::ZERO;
        self.warmup_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tesser_core::{Interval, Symbol};

    fn candle(close: f64) -> Candle {
        Candle {
            symbol: Symbol::from("BTCUSDT"),
            interval: Interval::OneMinute,
            open: Decimal::from_f64_retain(close).unwrap(),
            high: Decimal::from_f64_retain(close + 5.0).unwrap(),
            low: Decimal::from_f64_retain(close - 5.0).unwrap(),
            close: Decimal::from_f64_retain(close).unwrap(),
            volume: Decimal::ONE,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn atr_warms_up() {
        let mut atr = Atr::new(3).unwrap();
        assert!(atr.next(candle(100.0)).is_none());
        assert!(atr.next(candle(101.0)).is_none());
        assert!(atr.next(candle(102.0)).is_some());
    }
}
