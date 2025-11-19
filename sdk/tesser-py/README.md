# tesser-py

`tesser-py` is the official Python SDK for authoring remote Tesser strategies. It exposes a Pythonic API on top of the StrategyService gRPC contract so that quantitative researchers can write strategies using familiar tools such as pandas and numpy while the Rust engine remains responsible for ingestion, risk management, and execution.

## Features

- Async-first strategy base class with optional sync helpers.
- Dataclass models for ticks, candles, signals, positions, and more.
- Automatic conversion to/from pandas and numpy when available.
- A resilient gRPC server runner with structured logging and graceful shutdown.
- Example strategies that show how to integrate with TA libraries.

## Layout

```
tesser/
├── models.py          # Dataclasses and enums
├── strategy.py        # Base strategy API
├── conversions.py     # Proto ↔ Python conversions
├── runner.py          # Async runner bootstrapping gRPC server
├── service.py         # StrategyService implementation
├── client.py          # Optional helper client for testing
├── utils/             # Decimal helpers and logging utilities
└── protos/            # Generated protobuf & gRPC stubs
```

## Quick start

```bash
cd sdk/tesser-py
python -m pip install -e .[data,dev]
python -m grpc_tools.protoc \
  -I protos \
  --python_out=tesser/protos \
  --grpc_python_out=tesser/protos \
  protos/tesser/rpc/v1/tesser.proto
```

```python
import asyncio
from tesser.strategy import Strategy
from tesser.runner import Runner
from tesser.models import Signal, SignalKind

class PyCross(Strategy):
    def __init__(self):
        super().__init__(name="py-cross", symbol="BTC-USD")

    async def on_tick(self, context, tick):
        if tick.price > tick.price.median:
            return [Signal(symbol=tick.symbol, kind=SignalKind.ENTER_LONG)]
        return []

if __name__ == "__main__":
    asyncio.run(Runner(PyCross()).serve())
```

See `examples/` for more complete samples.
