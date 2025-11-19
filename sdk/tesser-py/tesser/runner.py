from __future__ import annotations

import asyncio
import signal
from typing import Optional

import grpc

from .protos.tesser.rpc.v1 import tesser_pb2_grpc
from .service import StrategyServiceImpl
from .strategy import Strategy
from .utils.logging import configure_logging


class Runner:
    def __init__(self, strategy: Strategy, host: str = "0.0.0.0", port: int = 50051):
        self.strategy = strategy
        self.host = host
        self.port = port
        self._server: Optional[grpc.aio.Server] = None

    async def serve(self):
        configure_logging()
        server = grpc.aio.server()
        tesser_pb2_grpc.add_StrategyServiceServicer_to_server(
            StrategyServiceImpl(self.strategy), server
        )
        server.add_insecure_port(f"{self.host}:{self.port}")
        self._server = server
        await server.start()
        print(f"Strategy '{self.strategy.name}' listening on {self.host}:{self.port}")
        await self._graceful_wait(server)

    async def _graceful_wait(self, server: grpc.aio.Server):
        loop = asyncio.get_event_loop()
        stop_event = asyncio.Event()

        def _handle_stop(*_):
            stop_event.set()

        loop.add_signal_handler(signal.SIGINT, _handle_stop)
        loop.add_signal_handler(signal.SIGTERM, _handle_stop)
        await stop_event.wait()
        await server.stop(5)
