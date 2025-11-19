from __future__ import annotations

import abc
from typing import Iterable

from .models import (
    Candle,
    OrderBook,
    Fill,
    Signal,
    StrategyContext,
    StrategyInitResult,
    Tick,
)


class Strategy(abc.ABC):
    """Base class for Python strategies."""

    def __init__(self, name: str, symbol: str):
        self.name = name
        self.symbol = symbol

    def on_init(self, config: dict) -> StrategyInitResult:
        return StrategyInitResult(symbols=[self.symbol])

    async def on_tick(self, context: StrategyContext, tick: Tick) -> Iterable[Signal]:
        return []

    async def on_candle(self, context: StrategyContext, candle: Candle) -> Iterable[Signal]:
        return []

    async def on_order_book(
        self, context: StrategyContext, order_book: OrderBook
    ) -> Iterable[Signal]:
        return []

    async def on_fill(self, context: StrategyContext, fill: Fill) -> Iterable[Signal]:
        return []
