//! Relative Strength Index (RSI).

use std::marker::PhantomData;

use rust_decimal::Decimal;

use crate::core::{decimal_from_usize, Indicator, IndicatorError, Input};

/// Computes Wilder's RSI oscillator scaled between 0 and 100.
#[derive(Debug, Clone)]
pub struct Rsi<I = Decimal> {
    period: usize,
    divisor: Decimal,
    decay: Decimal,
    prev_value: Option<Decimal>,
    avg_gain: Option<Decimal>,
    avg_loss: Option<Decimal>,
    warmup_count: usize,
    gain_sum: Decimal,
    loss_sum: Decimal,
    marker: PhantomData<I>,
}

impl<I> Rsi<I>
where
    I: Input,
{
    /// Creates a new RSI with the provided period.
    pub fn new(period: usize) -> Result<Self, IndicatorError> {
        if period == 0 {
            return Err(IndicatorError::invalid_period("RSI", period));
        }

        Ok(Self {
            period,
            divisor: decimal_from_usize(period),
            decay: decimal_from_usize(period.saturating_sub(1)),
            prev_value: None,
            avg_gain: None,
            avg_loss: None,
            warmup_count: 0,
            gain_sum: Decimal::ZERO,
            loss_sum: Decimal::ZERO,
            marker: PhantomData,
        })
    }

    fn compute_rsi(avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
        if avg_loss.is_zero() {
            Decimal::from(100)
        } else if avg_gain.is_zero() {
            Decimal::ZERO
        } else {
            let rs = avg_gain / avg_loss;
            Decimal::from(100) - (Decimal::from(100) / (rs + Decimal::ONE))
        }
    }
}

impl<I> Indicator for Rsi<I>
where
    I: Input,
{
    type Input = I;
    type Output = Decimal;

    fn next(&mut self, input: Self::Input) -> Option<Self::Output> {
        let value = input.value();

        let prev = match self.prev_value {
            Some(prev) => prev,
            None => {
                self.prev_value = Some(value);
                return None;
            }
        };

        let change = value - prev;
        let gain = if change.is_sign_positive() {
            change
        } else {
            Decimal::ZERO
        };
        let loss = if change.is_sign_negative() {
            -change
        } else {
            Decimal::ZERO
        };

        self.prev_value = Some(value);

        if self.avg_gain.is_none() {
            self.warmup_count += 1;
            self.gain_sum += gain;
            self.loss_sum += loss;

            if self.warmup_count < self.period {
                return None;
            }

            let avg_gain = self.gain_sum / self.divisor;
            let avg_loss = self.loss_sum / self.divisor;
            self.avg_gain = Some(avg_gain);
            self.avg_loss = Some(avg_loss);
            return Some(Self::compute_rsi(avg_gain, avg_loss));
        }

        let avg_gain = if self.period == 1 {
            gain
        } else {
            ((self.avg_gain.unwrap() * self.decay) + gain) / self.divisor
        };
        let avg_loss = if self.period == 1 {
            loss
        } else {
            ((self.avg_loss.unwrap() * self.decay) + loss) / self.divisor
        };

        self.avg_gain = Some(avg_gain);
        self.avg_loss = Some(avg_loss);

        Some(Self::compute_rsi(avg_gain, avg_loss))
    }

    fn reset(&mut self) {
        self.prev_value = None;
        self.avg_gain = None;
        self.avg_loss = None;
        self.warmup_count = 0;
        self.gain_sum = Decimal::ZERO;
        self.loss_sum = Decimal::ZERO;
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::Rsi;
    use crate::Indicator;

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }

    #[test]
    fn waits_for_initial_window() {
        let mut rsi = Rsi::new(3).unwrap();
        assert_eq!(rsi.next(dec("1")), None);
        assert_eq!(rsi.next(dec("2")), None);
        assert_eq!(rsi.next(dec("3")), None);
        assert!(rsi.next(dec("2")).is_some());
    }

    #[test]
    fn computes_expected_values() {
        let mut rsi = Rsi::new(3).unwrap();
        let series = ["1", "2", "3", "2", "1", "2", "3", "4"];
        let mut outputs = Vec::new();
        for value in series {
            outputs.push(rsi.next(dec(value)));
        }

        let filtered: Vec<_> = outputs.into_iter().flatten().collect();
        let expected = [
            dec("66.66666666666666666666666667"),
            dec("44.44444444444444444444444444"),
            dec("62.96296296296296296296296298"),
            dec("75.30864197530864197530864198"),
            dec("83.53909465020576131687242799"),
        ];

        assert_eq!(filtered.len(), expected.len());
        for (lhs, rhs) in filtered.iter().zip(expected.iter()) {
            assert!((lhs - rhs).abs() <= dec("0.0000000001"));
        }
    }

    #[test]
    fn reset_clears_buffers() {
        let mut rsi = Rsi::new(2).unwrap();
        rsi.next(dec("1"));
        rsi.next(dec("2"));
        assert!(rsi.next(dec("3")).is_some());
        rsi.reset();
        assert_eq!(rsi.next(dec("3")), None);
    }

    #[test]
    fn constant_input_registers_as_overbought() {
        let mut rsi = Rsi::new(3).unwrap();
        for _ in 0..4 {
            rsi.next(dec("1"));
        }
        assert_eq!(rsi.next(dec("1")), Some(dec("100")));
    }
}
