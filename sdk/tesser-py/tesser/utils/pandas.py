from __future__ import annotations

from dataclasses import asdict
from typing import Iterable


def to_dataframe(items: Iterable[object]):  # pragma: no cover
    try:
        import pandas as pd
    except ImportError as exc:  # pragma: no cover
        raise RuntimeError(
            "pandas is not installed. Install tesser[data] extra to enable DataFrame exports."
        ) from exc

    rows = [asdict(item) for item in items]
    return pd.DataFrame(rows)
