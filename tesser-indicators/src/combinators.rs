//! Combinators that allow indicators to be chained together.

use crate::core::Indicator;

/// Chains two indicators together, feeding the output of the first into the second.
pub struct PipedIndicator<First, Second> {
    first: First,
    second: Second,
}

impl<First, Second> PipedIndicator<First, Second> {
    /// Creates a new piped indicator.
    pub fn new(first: First, second: Second) -> Self {
        Self { first, second }
    }
}

impl<First, Second> Indicator for PipedIndicator<First, Second>
where
    First: Indicator,
    Second: Indicator<Input = First::Output>,
{
    type Input = First::Input;
    type Output = Second::Output;

    fn next(&mut self, input: Self::Input) -> Option<Self::Output> {
        let intermediate = self.first.next(input)?;
        self.second.next(intermediate)
    }

    fn reset(&mut self) {
        self.first.reset();
        self.second.reset();
    }
}
