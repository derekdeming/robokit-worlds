# RoboKit Worlds

RoboKit Worlds is the engine-neutral foundation for turning physical scientific workflows into calibrated, executable worlds. A world is a versioned contract containing the environment, robot and instrument semantics, protocol, uncertain parameters, and quantitative success and failure verifiers.

The first benchmark is **Granular Dosing v0**: transfer a requested mass of safe granular material between vessels, measure the result, and determine whether simulation predicts held-out physical outcomes.

## What exists in the first runtime vertical slice

- Typed, versioned scientific-world definitions
- Boundary validation with aggregated diagnostics
- Immutable physical and simulated run manifests
- Runtime capabilities and bounded execution requests
- Deterministic outcome verification
- A `worldctl` CLI for schema generation, validation, execution, and evaluation
- A pinned MuJoCo 3.7 numerical worker behind a typed Rust adapter contract
- A deterministic hybrid granular-process model with seeded parameter sampling
- Atomic run bundles containing outcomes, verifier reports, and exact provenance
- Typed physical-observation ingestion and deterministic granular parameter calibration
- Reproducible calibrated-world packages containing source data, source world, assets, and fit report
- An executable Granular Dosing v0 world and MuJoCo binding
- An explicit ROS 2 adapter ownership boundary for the later physical-runtime cutover

## Quick start

```bash
cargo run -p worldctl -- validate-world \
  --world worlds/granular-dosing-v0/world.toml

cargo run -p worldctl -- evaluate \
  --world worlds/granular-dosing-v0/world.toml \
  --outcome worlds/granular-dosing-v0/passing-outcome.json

cargo run -p worldctl -- simulate-mujoco \
  --world worlds/granular-dosing-v0/world.toml \
  --seed 9 \
  --output runs/granular-seed-9

cargo run -p worldctl -- validate-manifest \
  --manifest runs/granular-seed-9/manifest.json

cargo run -p worldctl -- calibrate-granular \
  --world worlds/granular-dosing-v0/world.toml \
  --dataset worlds/granular-dosing-v0/calibration/synthetic-physical-dataset.json \
  --new-version 0.3.0 \
  --output /tmp/robokit-calibrated-world

cargo run -p worldctl -- validate-calibration-package \
  --package /tmp/robokit-calibrated-world

cargo run -p worldctl -- simulate-mujoco \
  --world /tmp/robokit-calibrated-world/world.toml \
  --binding worlds/granular-dosing-v0/mujoco.toml \
  --seed 9 \
  --output /tmp/robokit-calibrated-replay

cargo run -p worldctl -- validate-manifest \
  --manifest worlds/granular-dosing-v0/example-run-manifest.json

cargo run -p worldctl -- schema \
  --output /tmp/robokit-world-definition.schema.json
```

## Architecture

```text
apps/worldctl ─────────────┐
                          ├──> adapters/mujoco ──> runtime-api ──> world-domain
                          ├──> calibration ──────────────────────> world-domain
                          ├──> evaluation ───────────────────────> world-domain
                          └──> evidence ─────────────────────────> world-domain

adapters/ros2 ───────────────> runtime-api
```

The domain is independent of MuJoCo, ROS, HTTP, databases, and deployment infrastructure. See [Architecture](docs/architecture.md) and [ADR 0001](docs/decisions/0001-engine-neutral-domain.md).

The checked-in calibration dataset is explicitly synthetic and exists only to exercise the pipeline. It is not physical evidence and must not be used for scientific claims. See the [physical data collection protocol](docs/physical-data-collection.md) for the real handoff.

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets

cd adapters/mujoco/python
uv sync --frozen --group dev
uv run --frozen ruff check .
uv run --frozen pytest
```

This repository is private and proprietary. No license is granted for redistribution or external use.
