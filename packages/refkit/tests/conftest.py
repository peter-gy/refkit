from __future__ import annotations

import contextlib
import functools
import inspect
import sys
from collections.abc import Generator
from types import TracebackType
from typing import Any, Self
from unittest.mock import Mock, patch

import pytest


class MockPyodide:
    """Simulate enough of Pyodide for unit tests."""

    def __init__(self, **extra_modules: Any) -> None:
        self._extra_modules = extra_modules
        self._stack: contextlib.ExitStack | None = None

    def __enter__(self) -> Self:
        modules = {"pyodide": Mock(), **self._extra_modules}
        stack = contextlib.ExitStack()
        stack.__enter__()
        stack.enter_context(patch.object(sys, "platform", "emscripten"))
        stack.enter_context(patch.dict(sys.modules, modules))
        self._stack = stack
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_value: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        assert self._stack is not None
        self._stack.__exit__(exc_type, exc_value, traceback)
        self._stack = None

    def __call__(self, func: Any) -> Any:
        extra_modules = self._extra_modules
        if inspect.iscoroutinefunction(func):

            @functools.wraps(func)
            async def async_wrapper(*args: Any, **kwargs: Any) -> Any:
                with MockPyodide(**extra_modules):
                    return await func(*args, **kwargs)

            return async_wrapper

        @functools.wraps(func)
        def sync_wrapper(*args: Any, **kwargs: Any) -> Any:
            with MockPyodide(**extra_modules):
                return func(*args, **kwargs)

        return sync_wrapper


@pytest.fixture
def mock_pyodide() -> type[MockPyodide]:
    return MockPyodide


@pytest.fixture
def pyodide_env() -> Generator[None, None, None]:
    with MockPyodide():
        yield
