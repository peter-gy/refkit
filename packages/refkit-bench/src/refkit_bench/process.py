from __future__ import annotations

import os
import signal
import subprocess


def _stop(process: subprocess.Popen[str]) -> None:
    if os.name == "nt":
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True)
    else:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    process.communicate()


def execute(command: list[str], timeout: float) -> subprocess.CompletedProcess[str]:
    with subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        start_new_session=os.name != "nt",
    ) as process:
        try:
            stdout, stderr = process.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            _stop(process)
            raise TimeoutError(f"benchmark process exceeded {timeout:g} seconds") from None
        except BaseException:
            _stop(process)
            raise
        return subprocess.CompletedProcess(command, process.returncode, stdout, stderr)
