//! Moving Average Convergence Divergence indicator implementation.

use rust_decimal::Decimal;

use crate::core::{Indicator, IndicatorError};
use crate::indicators::ema::Ema;

/// MACD output (line, signal line, and histogram).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MacdOutput {
    /// MACD line value (fast EMA minus slow EMA).
    pub macd: Decimal,
    /// Signal line value (EMA of the MACD line).
    pub signal: Decimal,
    /// Histogram representing the distance between MACD and signal lines.
    pub histogram: Decimal,
}

/// Moving Average Convergence Divergence indicator.
pub struct Macd {
    fast: Ema<Decimal>,
    slow: Ema<Decimal>,
    signal: Ema,
    last_histogram: Option<Decimal>,
}

impl Macd {
    /// Create a MACD indicator with custom fast/slow/signal periods.
    pub fn new(
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
    ) -> Result<Self, IndicatorError> {
        if fast_period == 0 {
            return Err(IndicatorError::invalid_period("MACD", fast_period));
        }
        if slow_period == 0 {
            return Err(IndicatorError::invalid_period("MACD", slow_period));
        }
        if signal_period == 0 {
            return Err(IndicatorError::invalid_period("MACD", signal_period));
        }
        Ok(Self {
            fast: Ema::new(fast_period)?,
            slow: Ema::new(slow_period)?,
            signal: Ema::new(signal_period)?,
            last_histogram: None,
        })
    }
}

impl Indicator for Macd {
    type Input = Decimal;
    type Output = MacdOutput;

    fn next(&mut self, input: Self::Input) -> Option<Self::Output> {
        let fast = self.fast.next(input);
        let slow = self.slow.next(input);
        match (fast, slow) {
            (Some(fast_val), Some(slow_val)) => {
                let macd = fast_val - slow_val;
                if let Some(signal_line) = self.signal.next(macd) {
                    let histogram = macd - signal_line;
                    self.last_histogram = Some(histogram);
                    Some(MacdOutput {
                        macd,
                        signal: signal_line,
                        histogram,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.fast.reset();
        self.slow.reset();
        self.signal.reset();
        self.last_histogram = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macd_emits_after_warmup() {
        let mut macd = Macd::new(3, 6, 3).unwrap();
        for price in 1..=15 {
            macd.next(Decimal::from(price));
        }
        assert!(macd.next(Decimal::from(16)).is_some());
    }
}
