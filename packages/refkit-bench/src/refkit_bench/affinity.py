from __future__ import annotations

import os

import psutil


def cpu_list(value: str, count: int) -> tuple[int, ...]:
    cpus: set[int] = set()
    for part in value.split(","):
        first, separator, last = part.strip().partition("-")
        try:
            start = int(first)
            end = int(last) if separator else start
        except ValueError as exc:
            raise ValueError("CPU affinity expects indexes or ranges, such as 0,2-3") from exc
        if not 0 <= start <= end < count:
            raise ValueError(f"CPU affinity must select indexes from 0 through {count - 1}")
        cpus.update(range(start, end + 1))
    return tuple(sorted(cpus))


def configure(value: str | None) -> tuple[int, ...] | None:
    process = psutil.Process()
    if not hasattr(process, "cpu_affinity"):
        if value is not None:
            raise ValueError("CPU affinity is unavailable on this operating system")
        return None
    inherited = tuple(sorted(process.cpu_affinity()))
    requested = (
        inherited if value is None else cpu_list(value, psutil.cpu_count() or max(inherited) + 1)
    )
    if not requested:
        raise ValueError("the process has no available CPUs")
    process.cpu_affinity(list(requested))
    if tuple(sorted(process.cpu_affinity())) != requested:
        raise ValueError("the operating system did not apply the requested CPU affinity")
    return requested


def describe(cpus: tuple[int, ...] | None) -> str:
    return "unavailable" if cpus is None else ",".join(map(str, cpus))


def verify(cpus: tuple[int, ...] | None) -> None:
    if cpus is None:
        return
    allowed = set(cpus)
    owner = psutil.Process()
    for process in [owner, *owner.children(recursive=True)]:
        try:
            if not hasattr(process, "cpu_affinity"):
                raise RuntimeError("CPU affinity verification is unavailable")
            if not set(process.cpu_affinity()) <= allowed:
                raise RuntimeError("a benchmark process escaped its declared CPU affinity")
            if hasattr(os, "sched_getaffinity"):
                for thread in process.threads():
                    try:
                        if not os.sched_getaffinity(thread.id) <= allowed:
                            raise RuntimeError(
                                "a benchmark thread escaped its declared CPU affinity"
                            )
                    except ProcessLookupError:
                        continue
        except psutil.NoSuchProcess:
            continue
