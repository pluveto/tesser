from decimal import Decimal
from typing import Optional

from google.protobuf.timestamp_pb2 import Timestamp


def to_decimal(value: str) -> Decimal:
    return Decimal(value) if value else Decimal("0")


def from_decimal(value: Decimal) -> str:
    return format(value, "f")


def to_timestamp(dt) -> Timestamp:
    stamp = Timestamp()
    stamp.FromDatetime(dt)
    return stamp


def from_timestamp(stamp: Optional[Timestamp]):
    if stamp is None:
        return None
    return stamp.ToDatetime()
