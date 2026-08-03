# Copyright (c) 2026 RoboKit. All rights reserved.
"""Deterministic hybrid process model for granular dosing."""

from __future__ import annotations

from dataclasses import dataclass
from random import Random
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from robokit_mujoco.models import Distribution, GranularProcessSpec


@dataclass(frozen=True, slots=True)
class SampledProcessParameters:
    """One seed-resolved set of physical process parameters."""

    flow_rate: float
    discharge_delay: float
    residual_mass: float
    scale_noise_standard_deviation: float
    scale_settling_time: float
    initial_material_mass: float
    spill_tilt_threshold: float
    pour_onset_tilt_threshold: float

    @classmethod
    def sample(cls, specification: GranularProcessSpec, random: Random) -> SampledProcessParameters:
        """Resolve every bounded distribution exactly once for a run."""
        return cls(
            flow_rate=sample_distribution(specification.flow_rate, random),
            discharge_delay=sample_distribution(specification.discharge_delay, random),
            residual_mass=sample_distribution(specification.residual_mass, random),
            scale_noise_standard_deviation=sample_distribution(
                specification.scale_noise_standard_deviation,
                random,
            ),
            scale_settling_time=sample_distribution(specification.scale_settling_time, random),
            initial_material_mass=sample_distribution(specification.initial_material_mass, random),
            spill_tilt_threshold=sample_distribution(
                specification.spill_tilt_threshold,
                random,
            ),
            pour_onset_tilt_threshold=sample_distribution(
                specification.pour_onset_tilt_threshold,
                random,
            ),
        )


@dataclass(slots=True)
class GranularProcess:
    """Mutable process state advanced alongside rigid-body simulation."""

    parameters: SampledProcessParameters
    random: Random
    source_mass: float
    discharge_delay_remaining: float
    settled_duration: float = 0.0
    spill_reported: bool = False

    @classmethod
    def create(cls, specification: GranularProcessSpec, seed: int) -> GranularProcess:
        """Create deterministic state from a run seed."""
        random = Random(seed)  # noqa: S311 - reproducibility, not cryptographic security
        parameters = SampledProcessParameters.sample(specification, random)
        return cls(
            parameters=parameters,
            random=random,
            source_mass=parameters.initial_material_mass,
            discharge_delay_remaining=parameters.discharge_delay,
        )

    def observe_tilt(self, tilt: float) -> bool:
        """Update spill and settling state for a simulated pose."""
        if tilt >= self.parameters.pour_onset_tilt_threshold:
            self.settled_duration = 0.0
        if not self.spill_reported and tilt >= self.parameters.spill_tilt_threshold:
            self.spill_reported = True
            return True
        return False

    def advance_wait(self, duration: float, tilt: float) -> bool:
        """Advance flow or upright settling during an explicit wait action."""
        spilled = self.observe_tilt(tilt)
        if tilt >= self.parameters.pour_onset_tilt_threshold:
            active_duration = max(duration - self.discharge_delay_remaining, 0.0)
            self.discharge_delay_remaining = max(
                self.discharge_delay_remaining - duration,
                0.0,
            )
            available_mass = max(self.source_mass - self.parameters.residual_mass, 0.0)
            transferred = min(self.parameters.flow_rate * active_duration, available_mass)
            self.source_mass -= transferred
        else:
            self.discharge_delay_remaining = self.parameters.discharge_delay
            self.settled_duration += duration
        return spilled

    def measure_mass(self) -> tuple[float, bool]:
        """Return a noisy transferred-mass reading and its stability state."""
        transferred_mass = self.parameters.initial_material_mass - self.source_mass
        noise = self.random.gauss(0.0, self.parameters.scale_noise_standard_deviation)
        measured_mass = max(transferred_mass + noise, 0.0)
        is_stable = self.settled_duration >= self.parameters.scale_settling_time
        return measured_mass, is_stable


def sample_distribution(distribution: Distribution, random: Random) -> float:
    """Sample a validated bounded distribution."""
    if distribution.distribution == "fixed":
        return distribution.value
    return random.uniform(distribution.minimum, distribution.maximum)
