"""Entry points that delegate validation to the tools in the uv environment."""

import subprocess
import sys


def after_edit() -> int:
    return subprocess.call(["ruff", "check", "src", "tests"])


def turn_end() -> int:
    return subprocess.call(
        [sys.executable, "-m", "unittest", "discover", "-s", "tests"]
    )
