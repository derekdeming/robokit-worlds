# Copyright (c) 2026 RoboKit. All rights reserved.
"""Strict private wire contract shared with the Rust MuJoCo adapter."""

from __future__ import annotations

import uuid  # noqa: TC003 - Pydantic resolves this runtime annotation
from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, model_validator


class ContractModel(BaseModel):
    """Fail-closed base model for every private adapter payload."""

    model_config = ConfigDict(extra="forbid", allow_inf_nan=False)


class FixedDistribution(ContractModel):
    """Deterministic scalar parameter."""

    distribution: Literal["fixed"]
    value: float


class UniformDistribution(ContractModel):
    """Bounded uniform scalar parameter."""

    distribution: Literal["uniform"]
    minimum: float
    maximum: float

    @model_validator(mode="after")
    def validate_bounds(self) -> UniformDistribution:
        """Require a non-empty bounded interval."""
        if self.minimum >= self.maximum:
            msg = "uniform minimum must be less than maximum"
            raise ValueError(msg)
        return self


Distribution = Annotated[
    FixedDistribution | UniformDistribution,
    Field(discriminator="distribution"),
]


class ExecutionLimits(ContractModel):
    """Hard bounds for one worker invocation."""

    maximum_steps: int = Field(gt=0)
    wall_clock_timeout: float = Field(gt=0.0)


class Position(ContractModel):
    """World-frame position in meters."""

    x: float
    y: float
    z: float


class Quaternion(ContractModel):
    """Unit quaternion in canonical x-y-z-w order."""

    x: float
    y: float
    z: float
    w: float


class Pose(ContractModel):
    """World-frame pose."""

    position: Position
    orientation: Quaternion


class MoveToPose(ContractModel):
    """Absolute mocap pose motion."""

    action: Literal["move_to_pose"]
    actor_id: str = Field(min_length=1)
    target: Pose
    duration: float = Field(gt=0.0)


class RotateAboutAxis(ContractModel):
    """Actor-frame principal-axis rotation."""

    action: Literal["rotate_about_axis"]
    actor_id: str = Field(min_length=1)
    axis: Literal["x", "y", "z"]
    angle: float
    duration: float = Field(gt=0.0)


class Wait(ContractModel):
    """Dynamics and process-settling interval."""

    action: Literal["wait"]
    duration: float = Field(ge=0.0)


class MeasureMass(ContractModel):
    """Gravimetric observation request."""

    action: Literal["measure_mass"]
    instrument_id: str = Field(min_length=1)
    measurement_id: str = Field(min_length=1)


Action = Annotated[
    MoveToPose | RotateAboutAxis | Wait | MeasureMass,
    Field(discriminator="action"),
]


class ProtocolStep(ContractModel):
    """One ordered canonical protocol step."""

    id: str = Field(min_length=1)
    action: Action

    @model_validator(mode="before")
    @classmethod
    def unwrap_flattened_action(cls, value: object) -> object:
        """Map Rust's flattened action fields into the typed internal action."""
        if (
            not isinstance(value, dict)
            or "action" not in value
            or isinstance(value["action"], dict)
        ):
            return value
        action_name = value.get("action")
        action_fields = {key: item for key, item in value.items() if key != "id"}
        return {"id": value.get("id"), "action": {**action_fields, "action": action_name}}


class GranularProcessSpec(ContractModel):
    """Hybrid process parameters compiled by Rust."""

    flow_rate: Distribution
    discharge_delay: Distribution
    residual_mass: Distribution
    scale_noise_standard_deviation: Distribution
    scale_settling_time: Distribution
    initial_material_mass: Distribution
    spill_tilt_threshold: Distribution
    pour_onset_tilt_threshold: Distribution


class AdapterExecutionRequest(ContractModel):
    """Fully compiled worker request."""

    schema_version: Literal["1.0.0"]
    run_id: uuid.UUID
    attempt: int = Field(gt=0)
    seed: int = Field(ge=0, le=(2**64) - 1)
    limits: ExecutionLimits
    scene_path: str = Field(min_length=1)
    controlled_body: str = Field(min_length=1)
    forbidden_contact_geometries: list[str]
    protocol: list[ProtocolStep]
    process: GranularProcessSpec


class MassMeasurementEvent(ContractModel):
    """Typed mass observation returned to Rust."""

    observation: Literal["mass_measurement"] = "mass_measurement"
    measurement_id: str = Field(min_length=1)
    value: float = Field(ge=0.0)


class ScientificEvent(ContractModel):
    """Typed scientific or operational event returned to Rust."""

    observation: Literal["event"] = "event"
    event: Literal["spill", "collision", "safety_stop", "unstable_measurement"]


RuntimeEvent = Annotated[
    MassMeasurementEvent | ScientificEvent,
    Field(discriminator="observation"),
]


class ExecutionSummary(ContractModel):
    """Bounded execution accounting."""

    executed_steps: int = Field(ge=0)
    logical_duration: float = Field(ge=0.0)
    wall_clock_duration: float = Field(ge=0.0)


class AdapterExecutionResponse(ContractModel):
    """Successful atomic worker response."""

    schema_version: Literal["1.0.0"] = "1.0.0"
    engine_version: str = Field(min_length=1)
    events: list[RuntimeEvent]
    summary: ExecutionSummary
