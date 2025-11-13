//! Bollinger Bands indicator built on top of SMA and standard deviation.

use std::collections::VecDeque;
use std::marker::PhantomData;

use rust_decimal::{Decimal, MathematicalOps};

use crate::core::{decimal_from_usize, Indicator, IndicatorError, Input};

/// Output value of the Bollinger Bands indicator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollingerBandsOutput {
    /// Upper band (mean + k * std dev).
    pub upper: Decimal,
    /// Middle band (rolling mean).
    pub middle: Decimal,
    /// Lower band (mean - k * std dev).
    pub lower: Decimal,
}

/// Produces Bollinger Bands from a rolling window.
#[derive(Debug, Clone)]
pub struct BollingerBands<I = Decimal> {
    period: usize,
    divisor: Decimal,
    std_multiplier: Decimal,
    sum: Decimal,
    sum_of_squares: Decimal,
    window: VecDeque<Decimal>,
    marker: PhantomData<I>,
}

impl<I> BollingerBands<I>
where
    I: Input,
{
    /// Creates a new Bollinger Bands indicator.
    pub fn new(period: usize, std_multiplier: Decimal) -> Result<Self, IndicatorError> {
        if period == 0 {
            return Err(IndicatorError::invalid_period("BollingerBands", period));
        }
        if std_multiplier.is_sign_negative() {
            return Err(IndicatorError::invalid_parameter(
                "BollingerBands",
                "std_multiplier",
                std_multiplier,
            ));
        }

        Ok(Self {
            period,
            divisor: decimal_from_usize(period),
            std_multiplier,
            sum: Decimal::ZERO,
            sum_of_squares: Decimal::ZERO,
            window: VecDeque::with_capacity(period),
            marker: PhantomData,
        })
    }

    fn compute_bands(&self) -> BollingerBandsOutput {
        let mean = self.sum / self.divisor;
        let mean_of_squares = self.sum_of_squares / self.divisor;
        let mut variance = mean_of_squares - mean * mean;
        if variance.is_sign_negative() {
            variance = Decimal::ZERO;
        }
        let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);
        let offset = self.std_multiplier * std_dev;

        BollingerBandsOutput {
            upper: mean + offset,
            middle: mean,
            lower: mean - offset,
        }
    }
}

impl<I> Indicator for BollingerBands<I>
where
    I: Input,
{
    type Input = I;
    type Output = BollingerBandsOutput;

    fn next(&mut self, input: Self::Input) -> Option<Self::Output> {
        let value = input.value();
        self.window.push_back(value);
        self.sum += value;
        self.sum_of_squares += value * value;

        if self.window.len() > self.period {
            if let Some(oldest) = self.window.pop_front() {
                self.sum -= oldest;
                self.sum_of_squares -= oldest * oldest;
            }
        }

        if self.window.len() == self.period {
            Some(self.compute_bands())
        } else {
            None
        }
    }

    fn reset(&mut self) {
        self.sum = Decimal::ZERO;
        self.sum_of_squares = Decimal::ZERO;
        self.window.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rust_decimal::Decimal;

    use super::BollingerBands;
    use crate::Indicator;

    fn dec(value: &str) -> Decimal {
        Decimal::from_str(value).unwrap()
    }

    fn assert_close(lhs: Decimal, rhs: Decimal) {
        let tolerance = dec("0.00000001");
        assert!((lhs - rhs).abs() <= tolerance, "{lhs} != {rhs}");
    }

    #[test]
    fn computes_expected_bands() {
        let mut bb = BollingerBands::new(5, dec("2")).unwrap();
        let series = ["10", "11", "12", "13", "14"];
        let mut output = None;
        for value in series {
            output = bb.next(dec(value));
        }

        let bands = output.unwrap();
        assert_close(bands.middle, dec("12"));
        assert_close(bands.upper, dec("14.82842712"));
        assert_close(bands.lower, dec("9.17157288"));
    }

    #[test]
    fn respects_reset() {
        let mut bb = BollingerBands::new(2, dec("1")).unwrap();
        bb.next(dec("1"));
        let first = bb.next(dec("3")).unwrap();
        assert_close(first.middle, dec("2"));
        bb.reset();
        assert_eq!(bb.next(dec("3")), None);
    }

    #[test]
    fn rejects_negative_multiplier() {
        let err = BollingerBands::<Decimal>::new(5, dec("-1")).unwrap_err();
        assert!(matches!(
            err,
            crate::IndicatorError::InvalidParameter { .. }
        ));
    }
}
