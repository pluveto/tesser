use std::collections::HashMap;
use std::sync::Arc;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tesser_core::{Price, Quantity, Side};

fn zero_decimal() -> Decimal {
    Decimal::ZERO
}

/// Describes the role of a fill relative to the order book.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiquidityRole {
    Maker,
    Taker,
}

/// Context passed to fee models when computing charges for a fill.
#[derive(Clone, Debug)]
pub struct FeeContext<'a> {
    pub symbol: &'a str,
    pub side: Side,
    pub role: LiquidityRole,
}

/// Trait implemented by any structure capable of computing fill fees.
pub trait FeeModel: Send + Sync {
    /// Returns the absolute fee charged for the provided fill context.
    fn fee(&self, ctx: FeeContext<'_>, price: Price, quantity: Quantity) -> Decimal;
}

#[derive(Clone, Copy, Debug)]
struct FeePair {
    maker_bps: Decimal,
    taker_bps: Decimal,
}

impl FeePair {
    fn rate(&self, role: LiquidityRole) -> Decimal {
        match role {
            LiquidityRole::Maker => self.maker_bps,
            LiquidityRole::Taker => self.taker_bps,
        }
    }
}

#[derive(Clone, Debug)]
struct ScheduleFeeModel {
    default: FeePair,
    overrides: HashMap<String, FeePair>,
}

impl ScheduleFeeModel {
    fn new(default: FeePair, overrides: HashMap<String, FeePair>) -> Self {
        Self { default, overrides }
    }

    fn pair_for<'a>(&'a self, symbol: &str) -> &'a FeePair {
        self.overrides.get(symbol).unwrap_or(&self.default)
    }
}

impl FeeModel for ScheduleFeeModel {
    fn fee(&self, ctx: FeeContext<'_>, price: Price, quantity: Quantity) -> Decimal {
        let pair = self.pair_for(ctx.symbol);
        let bps = pair.rate(ctx.role).max(Decimal::ZERO);
        if bps.is_zero() || quantity.is_zero() || price.is_zero() {
            return Decimal::ZERO;
        }
        let notional = price * quantity.abs();
        (bps / Decimal::from(10_000)) * notional
    }
}

/// Serializable configuration describing maker/taker rates per market.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeeScheduleConfig {
    #[serde(default = "zero_decimal")]
    pub default_maker_bps: Decimal,
    #[serde(default = "zero_decimal")]
    pub default_taker_bps: Decimal,
    #[serde(default)]
    pub markets: HashMap<String, MarketFeeConfig>,
}

impl FeeScheduleConfig {
    /// Build a fee schedule that uses identical maker/taker rates for every symbol.
    pub fn flat(bps: Decimal) -> Self {
        Self {
            default_maker_bps: bps,
            default_taker_bps: bps,
            markets: HashMap::new(),
        }
    }

    /// Build a schedule using explicit maker/taker defaults.
    pub fn with_defaults(maker_bps: Decimal, taker_bps: Decimal) -> Self {
        Self {
            default_maker_bps: maker_bps,
            default_taker_bps: taker_bps,
            markets: HashMap::new(),
        }
    }

    /// Convert this config into a fee model handle.
    pub fn build_model(&self) -> Arc<dyn FeeModel> {
        let default = FeePair {
            maker_bps: self.default_maker_bps,
            taker_bps: self.default_taker_bps,
        };
        let overrides = self
            .markets
            .iter()
            .map(|(symbol, cfg)| {
                (
                    symbol.clone(),
                    FeePair {
                        maker_bps: cfg.maker_bps,
                        taker_bps: cfg.taker_bps,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        Arc::new(ScheduleFeeModel::new(default, overrides))
    }
}

impl Default for FeeScheduleConfig {
    fn default() -> Self {
        Self {
            default_maker_bps: Decimal::ZERO,
            default_taker_bps: Decimal::ZERO,
            markets: HashMap::new(),
        }
    }
}

/// Per-market override describing maker/taker basis points.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketFeeConfig {
    pub maker_bps: Decimal,
    pub taker_bps: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromPrimitive;

    #[test]
    fn schedule_fee_model_applies_overrides() {
        let mut markets = HashMap::new();
        markets.insert(
            "BTCUSDT".into(),
            MarketFeeConfig {
                maker_bps: Decimal::from_f64(0.1).unwrap(),
                taker_bps: Decimal::from_f64(0.2).unwrap(),
            },
        );
        let cfg = FeeScheduleConfig {
            default_maker_bps: Decimal::from_f64(0.01).unwrap(),
            default_taker_bps: Decimal::from_f64(0.02).unwrap(),
            markets,
        };
        let model = cfg.build_model();
        let maker_fee = model.fee(
            FeeContext {
                symbol: "BTCUSDT",
                side: Side::Buy,
                role: LiquidityRole::Maker,
            },
            Decimal::from(25_000),
            Decimal::from_f64(0.5).unwrap(),
        );
        assert_eq!(
            maker_fee,
            Decimal::from_f64(0.125).unwrap(), // 0.1 bps * 12_500 notional
        );
        let taker_fee = model.fee(
            FeeContext {
                symbol: "ETHUSDT",
                side: Side::Sell,
                role: LiquidityRole::Taker,
            },
            Decimal::from(2_000),
            Decimal::ONE,
        );
        let expected_taker =
            Decimal::from(2_000) * (Decimal::from_f64(0.02).unwrap() / Decimal::from(10_000));
        assert_eq!(taker_fee, expected_taker);
    }
}
