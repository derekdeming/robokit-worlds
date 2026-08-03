# Copyright (c) 2026 RoboKit. All rights reserved.
"""File-based command-line boundary for the MuJoCo worker."""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from pydantic import ValidationError

from robokit_mujoco.models import AdapterExecutionRequest
from robokit_mujoco.runtime import execute


def main() -> int:
    """Validate a request, execute it, and atomically publish its response."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--request", required=True, type=Path)
    parser.add_argument("--response", required=True, type=Path)
    arguments = parser.parse_args()

    try:
        request = AdapterExecutionRequest.model_validate_json(arguments.request.read_bytes())
        response = execute(request)
        temporary_response = arguments.response.with_suffix(".tmp")
        temporary_response.write_text(
            response.model_dump_json(indent=2),
            encoding="utf-8",
        )
        temporary_response.replace(arguments.response)
    except (OSError, RuntimeError, ValidationError, ValueError) as error:
        sys.stderr.write(f"robokit-mujoco: {error}\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
