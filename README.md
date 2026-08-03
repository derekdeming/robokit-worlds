# RoboKit Worlds

RoboKit Worlds is the engine-neutral foundation for turning physical scientific workflows into calibrated, executable worlds. A world is a versioned contract containing the environment, robot and instrument semantics, protocol, uncertain parameters, and quantitative success and failure verifiers.

The first benchmark is **Granular Dosing v0**: transfer a requested mass of safe granular material between vessels, measure the result, and determine whether simulation predicts held-out physical outcomes.

## What exists in this initial vertical slice

- Typed, versioned scientific-world definitions
- Boundary validation with aggregated diagnostics
- Immutable physical and simulated run manifests
- Runtime capabilities and bounded execution requests
- Deterministic outcome verification
- A `worldctl` CLI for schema generation, validation, and evaluation
- An executable Granular Dosing v0 example
- Explicit MuJoCo and ROS 2 adapter ownership boundaries

## Quick start

```bash
cargo run -p worldctl -- validate-world \
  --world worlds/granular-dosing-v0/world.toml

cargo run -p worldctl -- evaluate \
  --world worlds/granular-dosing-v0/world.toml \
  --outcome worlds/granular-dosing-v0/passing-outcome.json

cargo run -p worldctl -- validate-manifest \
  --manifest worlds/granular-dosing-v0/example-run-manifest.json

cargo run -p worldctl -- schema \
  --output /tmp/robokit-world-definition.schema.json
```

## Architecture

```text
apps/worldctl ─────────────┐
                          ├──> evaluation ──> world-domain
                          ├──> evidence ─────> world-domain
                          └──> runtime-api ──> world-domain

adapters/mujoco ─────────────> runtime-api
adapters/ros2 ───────────────> runtime-api
```

The domain is independent of MuJoCo, ROS, HTTP, databases, and deployment infrastructure. See [Architecture](docs/architecture.md) and [ADR 0001](docs/decisions/0001-engine-neutral-domain.md).

## Development

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```

This repository is private and proprietary. No license is granted for redistribution or external use.
