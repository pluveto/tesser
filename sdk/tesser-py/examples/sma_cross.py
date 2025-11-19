import asyncio
from decimal import Decimal

from tesser.models import Signal, SignalKind
from tesser.runner import Runner
from tesser.strategy import Strategy


class SmaCross(Strategy):
    def __init__(self):
        super().__init__(name="py-sma-cross", symbol="BTC-USD")

    async def on_tick(self, context, tick):
        threshold = Decimal("50000")
        if tick.price > threshold:
            return [Signal(symbol=tick.symbol, kind=SignalKind.ENTER_LONG)]
        return []


async def main():
    strategy = SmaCross()
    await Runner(strategy).serve()


if __name__ == "__main__":
    asyncio.run(main())
