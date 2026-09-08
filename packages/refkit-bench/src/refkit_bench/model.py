from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from math import isfinite
from time import perf_counter

MetadataValue = str | int | float


def _close() -> None:
    pass


@dataclass
class Prepared:
    operation: Callable[[], object]
    check: Callable[[object], None]
    metadata: dict[str, MetadataValue] = field(default_factory=dict)
    close: Callable[[], None] = _close
    measure: Callable[[int], tuple[float, object]] | None = None

    def time_batch(self, loops: int) -> float:
        if loops < 1:
            raise ValueError("a measured batch requires at least one operation")
        if self.measure is not None:
            seconds, result = self.measure(loops)
        else:
            operation = self.operation
            result: object = None
            start = perf_counter()
            for _ in range(loops):
                result = operation()
            seconds = perf_counter() - start
        if not isfinite(seconds) or seconds <= 0:
            raise ValueError("a measured batch must return finite positive elapsed seconds")
        self.check(result)
        return seconds
