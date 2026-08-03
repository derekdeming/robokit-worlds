# Copyright (c) 2026 RoboKit. All rights reserved.
"""Focused tests for deterministic granular process behavior."""

from robokit_mujoco.models import FixedDistribution, GranularProcessSpec
from robokit_mujoco.process_model import GranularProcess

EXPECTED_TRANSFERRED_MASS = 90.0


def process_specification() -> GranularProcessSpec:
    """Build a deterministic process contract for unit tests."""
    return GranularProcessSpec(
        flow_rate=FixedDistribution(distribution="fixed", value=50.0),
        discharge_delay=FixedDistribution(distribution="fixed", value=0.2),
        residual_mass=FixedDistribution(distribution="fixed", value=2.0),
        scale_noise_standard_deviation=FixedDistribution(distribution="fixed", value=0.0),
        scale_settling_time=FixedDistribution(distribution="fixed", value=1.0),
        initial_material_mass=FixedDistribution(distribution="fixed", value=250.0),
        spill_tilt_threshold=FixedDistribution(distribution="fixed", value=1.2),
        pour_onset_tilt_threshold=FixedDistribution(distribution="fixed", value=0.35),
    )


def test_wait_advances_delayed_flow_and_measurement() -> None:
    """Flow starts after the sampled delay and preserves residual mass."""
    process = GranularProcess.create(process_specification(), seed=7)

    process.advance_wait(duration=2.0, tilt=1.0)
    process.advance_wait(duration=1.0, tilt=0.0)
    measurement, stable = process.measure_mass()

    assert measurement == EXPECTED_TRANSFERRED_MASS
    assert stable


def test_spill_is_reported_once() -> None:
    """A persistent threshold violation produces one event."""
    process = GranularProcess.create(process_specification(), seed=7)

    assert process.observe_tilt(1.2)
    assert not process.observe_tilt(1.3)
