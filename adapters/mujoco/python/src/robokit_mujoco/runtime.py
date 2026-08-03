# Copyright (c) 2026 RoboKit. All rights reserved.
"""MuJoCo stepping and protocol execution."""

from __future__ import annotations

import math
import time
from dataclasses import dataclass, field
from typing import TYPE_CHECKING

import mujoco
import numpy as np
from numpy.typing import NDArray

from robokit_mujoco.models import (
    AdapterExecutionRequest,
    AdapterExecutionResponse,
    ExecutionSummary,
    MassMeasurementEvent,
    MeasureMass,
    MoveToPose,
    RotateAboutAxis,
    RuntimeEvent,
    ScientificEvent,
    Wait,
)
from robokit_mujoco.process_model import GranularProcess

if TYPE_CHECKING:
    from collections.abc import Callable

Vector = NDArray[np.float64]
LINEAR_INTERPOLATION_DOT_THRESHOLD = 0.9995


class RuntimeContractError(RuntimeError):
    """The compiled request does not match the loaded MuJoCo model."""


class RuntimeLimitError(RuntimeError):
    """Execution exceeded an explicit request bound."""


@dataclass(slots=True)
class ExecutionState:
    """All mutable state owned by one worker invocation."""

    request: AdapterExecutionRequest
    model: mujoco.MjModel
    data: mujoco.MjData
    mocap_id: int
    controlled_geometry_ids: frozenset[int]
    forbidden_geometry_ids: frozenset[int]
    process: GranularProcess
    deadline: float
    base_timestep: float
    executed_steps: int = 0
    logical_duration: float = 0.0
    collision_reported: bool = False
    events: list[RuntimeEvent] = field(default_factory=list)

    def step(self, duration: float, *, process_wait: bool) -> None:
        """Advance one bounded MuJoCo step and update scientific state."""
        self.check_limits()
        if self.executed_steps >= self.request.limits.maximum_steps:
            msg = f"maximum_steps exceeded: {self.request.limits.maximum_steps}"
            raise RuntimeLimitError(msg)
        self.model.opt.timestep = duration
        mujoco.mj_step(self.model, self.data)
        self.executed_steps += 1
        self.logical_duration += duration

        tilt = quaternion_tilt(self.current_quaternion())
        spilled = (
            self.process.advance_wait(duration, tilt)
            if process_wait
            else self.process.observe_tilt(tilt)
        )
        if spilled:
            self.events.append(ScientificEvent(event="spill"))
        if not self.collision_reported and self.has_forbidden_contact():
            self.collision_reported = True
            self.events.append(ScientificEvent(event="collision"))
        self.check_limits()

    def check_limits(self) -> None:
        """Enforce the monotonic wall-clock deadline."""
        if time.monotonic() > self.deadline:
            msg = f"wall_clock_timeout exceeded: {self.request.limits.wall_clock_timeout}s"
            raise RuntimeLimitError(msg)

    def current_quaternion(self) -> Vector:
        """Read the controlled mocap quaternion in x-y-z-w order."""
        w, x, y, z = self.data.mocap_quat[self.mocap_id]
        return np.asarray([x, y, z, w], dtype=np.float64)

    def set_pose(self, position: Vector, quaternion: Vector) -> None:
        """Write a normalized mocap pose in MuJoCo's w-x-y-z order."""
        normalized = normalize_quaternion(quaternion)
        self.data.mocap_pos[self.mocap_id] = position
        self.data.mocap_quat[self.mocap_id] = np.asarray(
            [normalized[3], normalized[0], normalized[1], normalized[2]],
            dtype=np.float64,
        )

    def has_forbidden_contact(self) -> bool:
        """Detect contact between the controlled body and bound forbidden geometries."""
        for index in range(self.data.ncon):
            contact = self.data.contact[index]
            pair = {int(contact.geom1), int(contact.geom2)}
            if pair & self.controlled_geometry_ids and pair & self.forbidden_geometry_ids:
                return True
        return False


def execute(request: AdapterExecutionRequest) -> AdapterExecutionResponse:
    """Load the bound scene and execute one fully compiled protocol."""
    started_at = time.monotonic()
    state = create_execution_state(request, started_at)
    state.check_limits()
    mujoco.mj_forward(state.model, state.data)

    for protocol_step in request.protocol:
        state.check_limits()
        action = protocol_step.action
        if isinstance(action, MoveToPose):
            execute_move(state, action)
        elif isinstance(action, RotateAboutAxis):
            execute_rotation(state, action)
        elif isinstance(action, Wait):
            execute_wait(state, action.duration)
        elif isinstance(action, MeasureMass):
            record_mass_measurement(state, action)
        state.check_limits()

    state.model.opt.timestep = state.base_timestep
    wall_clock_duration = time.monotonic() - started_at
    return AdapterExecutionResponse(
        engine_version=mujoco.__version__,
        events=state.events,
        summary=ExecutionSummary(
            executed_steps=state.executed_steps,
            logical_duration=state.logical_duration,
            wall_clock_duration=wall_clock_duration,
        ),
    )


def create_execution_state(
    request: AdapterExecutionRequest,
    started_at: float,
) -> ExecutionState:
    """Load and validate every model binding before protocol execution."""
    model = mujoco.MjModel.from_xml_path(request.scene_path)
    data = mujoco.MjData(model)
    body_id = required_name_id(model, mujoco.mjtObj.mjOBJ_BODY, request.controlled_body)
    mocap_id = int(model.body_mocapid[body_id])
    if mocap_id < 0:
        msg = f"controlled body `{request.controlled_body}` is not a mocap body"
        raise RuntimeContractError(msg)

    controlled_geometry_ids = frozenset(
        index for index, owner in enumerate(model.geom_bodyid) if int(owner) == body_id
    )
    if not controlled_geometry_ids:
        msg = f"controlled body `{request.controlled_body}` has no geometries"
        raise RuntimeContractError(msg)
    forbidden_geometry_ids = frozenset(
        required_name_id(model, mujoco.mjtObj.mjOBJ_GEOM, name)
        for name in request.forbidden_contact_geometries
    )
    overlapping_geometry_ids = controlled_geometry_ids & forbidden_geometry_ids
    if overlapping_geometry_ids:
        message = "forbidden contact geometries must not belong to the controlled body"
        raise RuntimeContractError(message)
    base_timestep = float(model.opt.timestep)
    if not math.isfinite(base_timestep) or base_timestep <= 0.0:
        message = "MuJoCo model timestep must be positive and finite"
        raise RuntimeContractError(message)

    return ExecutionState(
        request=request,
        model=model,
        data=data,
        mocap_id=mocap_id,
        controlled_geometry_ids=controlled_geometry_ids,
        forbidden_geometry_ids=forbidden_geometry_ids,
        process=GranularProcess.create(request.process, request.seed),
        deadline=started_at + request.limits.wall_clock_timeout,
        base_timestep=base_timestep,
    )


def record_mass_measurement(state: ExecutionState, action: MeasureMass) -> None:
    """Record stability and the typed gravimetric observation in order."""
    measurement, stable = state.process.measure_mass()
    if not stable:
        state.events.append(ScientificEvent(event="unstable_measurement"))
    state.events.append(
        MassMeasurementEvent(
            measurement_id=action.measurement_id,
            value=measurement,
        ),
    )


def execute_move(state: ExecutionState, action: MoveToPose) -> None:
    """Interpolate an absolute world-frame pose."""
    start_position = state.data.mocap_pos[state.mocap_id].copy()
    start_quaternion = state.current_quaternion()
    target_position = np.asarray(
        [action.target.position.x, action.target.position.y, action.target.position.z],
        dtype=np.float64,
    )
    target_quaternion = np.asarray(
        [
            action.target.orientation.x,
            action.target.orientation.y,
            action.target.orientation.z,
            action.target.orientation.w,
        ],
        dtype=np.float64,
    )

    def update_pose(fraction: float) -> None:
        position = start_position + ((target_position - start_position) * fraction)
        state.set_pose(position, slerp(start_quaternion, target_quaternion, fraction))

    execute_timed(state, action.duration, update_pose, process_wait=False)


def execute_rotation(state: ExecutionState, action: RotateAboutAxis) -> None:
    """Interpolate an actor-frame relative rotation."""
    position = state.data.mocap_pos[state.mocap_id].copy()
    start_quaternion = state.current_quaternion()
    target_quaternion = quaternion_multiply(
        start_quaternion,
        axis_angle_quaternion(action.axis, action.angle),
    )

    def update_pose(fraction: float) -> None:
        state.set_pose(position, slerp(start_quaternion, target_quaternion, fraction))

    execute_timed(state, action.duration, update_pose, process_wait=False)


def execute_wait(state: ExecutionState, duration: float) -> None:
    """Advance dynamics and the process model without changing the command pose."""
    execute_timed(state, duration, lambda _fraction: None, process_wait=True)


def execute_timed(
    state: ExecutionState,
    duration: float,
    update_command: Callable[[float], None],
    *,
    process_wait: bool,
) -> None:
    """Execute an exactly timed command using bounded model steps."""
    starting_logical_duration = state.logical_duration
    elapsed = 0.0
    while elapsed < duration:
        step_duration = min(state.base_timestep, duration - elapsed)
        next_elapsed = elapsed + step_duration
        update_command(next_elapsed / duration)
        state.step(step_duration, process_wait=process_wait)
        elapsed = next_elapsed
    state.logical_duration = starting_logical_duration + duration


def required_name_id(model: mujoco.MjModel, object_type: mujoco.mjtObj, name: str) -> int:
    """Resolve one required MuJoCo name without silent sentinel handling."""
    object_id = int(mujoco.mj_name2id(model, object_type, name))
    if object_id < 0:
        msg = f"MuJoCo object `{name}` of type {object_type.name} does not exist"
        raise RuntimeContractError(msg)
    return object_id


def normalize_quaternion(quaternion: Vector) -> Vector:
    """Normalize a quaternion and reject degenerate input."""
    norm = float(np.linalg.norm(quaternion))
    if not math.isfinite(norm) or norm <= 0.0:
        message = "quaternion norm must be positive and finite"
        raise RuntimeContractError(message)
    return quaternion / norm


def quaternion_multiply(left: Vector, right: Vector) -> Vector:
    """Multiply x-y-z-w quaternions."""
    lx, ly, lz, lw = left
    rx, ry, rz, rw = right
    return normalize_quaternion(
        np.asarray(
            [
                (lw * rx) + (lx * rw) + (ly * rz) - (lz * ry),
                (lw * ry) - (lx * rz) + (ly * rw) + (lz * rx),
                (lw * rz) + (lx * ry) - (ly * rx) + (lz * rw),
                (lw * rw) - (lx * rx) - (ly * ry) - (lz * rz),
            ],
            dtype=np.float64,
        ),
    )


def axis_angle_quaternion(axis: str, angle: float) -> Vector:
    """Create an x-y-z-w quaternion for a principal-axis rotation."""
    components = {"x": 0, "y": 1, "z": 2}
    quaternion = np.zeros(4, dtype=np.float64)
    quaternion[components[axis]] = math.sin(angle / 2.0)
    quaternion[3] = math.cos(angle / 2.0)
    return quaternion


def slerp(start: Vector, target: Vector, fraction: float) -> Vector:
    """Spherically interpolate normalized x-y-z-w quaternions."""
    start = normalize_quaternion(start)
    target = normalize_quaternion(target)
    dot = float(np.dot(start, target))
    if dot < 0.0:
        target = -target
        dot = -dot
    dot = min(dot, 1.0)
    if dot > LINEAR_INTERPOLATION_DOT_THRESHOLD:
        return normalize_quaternion(start + (fraction * (target - start)))
    angle = math.acos(dot)
    scale = math.sin(angle)
    return (
        (math.sin((1.0 - fraction) * angle) / scale * start)
        + (math.sin(fraction * angle) / scale * target)
    )


def quaternion_tilt(quaternion: Vector) -> float:
    """Return the angle between the actor and world positive-z axes."""
    x, y, _z, _w = normalize_quaternion(quaternion)
    local_z_world_z = 1.0 - (2.0 * ((x * x) + (y * y)))
    return math.acos(min(max(float(local_z_world_z), -1.0), 1.0))
