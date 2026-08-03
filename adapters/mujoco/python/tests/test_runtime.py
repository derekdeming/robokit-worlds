# Copyright (c) 2026 RoboKit. All rights reserved.
"""Integration test for the strict request contract and MuJoCo runtime."""

from pathlib import Path

import pytest

from robokit_mujoco.models import AdapterExecutionRequest, MassMeasurementEvent
from robokit_mujoco.runtime import execute

REPOSITORY_ROOT = Path(__file__).parents[4]
SCENE_PATH = REPOSITORY_ROOT / "worlds/granular-dosing-v0/assets/workcell.xml"
EXPECTED_LOGICAL_DURATION = 3.2
EXPECTED_EXECUTED_STEPS = 1600


def test_executes_flattened_rust_protocol_contract() -> None:
    """The pinned engine executes Rust's flattened action representation end to end."""
    request = AdapterExecutionRequest.model_validate(
        {
            "schema_version": "1.0.0",
            "run_id": "00000000-0000-0000-0000-000000000001",
            "attempt": 1,
            "seed": 7,
            "limits": {"maximum_steps": 5000, "wall_clock_timeout": 10.0},
            "scene_path": str(SCENE_PATH),
            "controlled_body": "dosing_tool",
            "forbidden_contact_geometries": ["target_vessel"],
            "protocol": [
                {
                    "id": "rotate",
                    "action": "rotate_about_axis",
                    "actor_id": "robot",
                    "axis": "y",
                    "angle": 1.0,
                    "duration": 0.1,
                },
                {"id": "pour", "action": "wait", "duration": 2.0},
                {
                    "id": "upright",
                    "action": "rotate_about_axis",
                    "actor_id": "robot",
                    "axis": "y",
                    "angle": -1.0,
                    "duration": 0.1,
                },
                {"id": "settle", "action": "wait", "duration": 1.0},
                {
                    "id": "measure",
                    "action": "measure_mass",
                    "instrument_id": "scale",
                    "measurement_id": "transferred_mass",
                },
            ],
            "process": {
                "flow_rate": {"distribution": "fixed", "value": 50.0},
                "discharge_delay": {"distribution": "fixed", "value": 0.2},
                "residual_mass": {"distribution": "fixed", "value": 2.0},
                "scale_noise_standard_deviation": {"distribution": "fixed", "value": 0.0},
                "scale_settling_time": {"distribution": "fixed", "value": 1.0},
                "initial_material_mass": {"distribution": "fixed", "value": 250.0},
                "spill_tilt_threshold": {"distribution": "fixed", "value": 1.2},
                "pour_onset_tilt_threshold": {"distribution": "fixed", "value": 0.35},
            },
        },
    )

    response = execute(request)

    assert response.engine_version == "3.7.0"
    assert response.summary.logical_duration == pytest.approx(EXPECTED_LOGICAL_DURATION)
    assert response.summary.executed_steps == EXPECTED_EXECUTED_STEPS
    assert len(response.events) == 1
    measurement = response.events[0]
    assert isinstance(measurement, MassMeasurementEvent)
    assert measurement.measurement_id == "transferred_mass"
    assert measurement.value == pytest.approx(90.0)
