//! Simple Moving Average (SMA).

use std::collections::VecDeque;
use std::marker::PhantomData;

use rust_decimal::Decimal;

use crate::core::{decimal_from_usize, Indicator, IndicatorError, Input};

/// Computes the arithmetic mean over a rolling window.
#[derive(Debug, Clone)]
pub struct Sma<I = Decimal> {
    period: usize,
    divisor: Decimal,
    sum: Decimal,
    window: VecDeque<Decimal>,
    marker: PhantomData<I>,
}

impl<I> Sma<I>
where
    I: Input,
{
    /// Creates a new SMA with the provided period.
    pub fn new(period: usize) -> Result<Self, IndicatorError> {
        if period == 0 {
            return Err(IndicatorError::invalid_period("SMA", period));
        }

        Ok(Self {
            period,
            divisor: decimal_from_usize(period),
            sum: Decimal::ZERO,
            window: VecDeque::with_capacity(period),
            marker: PhantomData,
        })
    }

    /// Returns the configured lookback period.
    pub fn period(&self) -> usize {
        self.period
    }
}

impl<I> Indicator for Sma<I>
where
    I: Input,
{
    type Input = I;
    type Output = Decimal;

    fn next(&mut self, input: Self::Input) -> Option<Self::Output> {
        let value = input.value();
        self.window.push_back(value);
        self.sum += value;

        if self.window.len() > self.period {
            if let Some(oldest) = self.window.pop_front() {
                self.sum -= oldest;
            }
        }

        if self.window.len() == self.period {
            Some(self.sum / self.divisor)
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.sum = Decimal::ZERO;
        self.window.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::Sma;
    use crate::Indicator;

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }

    #[test]
    fn waits_for_full_window() {
        let mut sma = Sma::new(3).unwrap();
        assert_eq!(sma.next(dec("1")), None);
        assert_eq!(sma.next(dec("2")), None);
        assert_eq!(sma.next(dec("3")), Some(dec("2")));
    }

    #[test]
    fn rolls_forward_in_constant_time() {
        let mut sma = Sma::new(3).unwrap();
        sma.next(dec("1"));
        sma.next(dec("2"));
        sma.next(dec("3"));
        assert_eq!(sma.next(dec("4")), Some(dec("3")));
        assert_eq!(sma.next(dec("5")), Some(dec("4")));
    }

    #[test]
    fn reset_clears_internal_state() {
        let mut sma = Sma::new(2).unwrap();
        sma.next(dec("5"));
        sma.next(dec("7"));
        assert_eq!(sma.next(dec("9")), Some(dec("8")));
        sma.reset();
        assert_eq!(sma.next(dec("9")), None);
    }
}
