# Granular Dosing v0 benchmark

## Hypothesis

A calibrated hybrid world can predict the transferred-mass distribution and dosing failures of held-out physical pouring conditions better than an uncalibrated physics-only baseline.

## Task

Transfer a requested mass of nonhazardous granular material from a source vessel to a target vessel, place or retain the target on a digital scale, and evaluate the result after settling.

## Controlled variables

- Requested mass
- Initial source fill mass
- Source and target vessel geometry
- Tilt angle, angular velocity, and hold duration
- Gripper pose
- Material batch
- Scale sampling and settling behavior

## Recorded observations

- Commanded and observed robot state
- Source and target poses
- Scale measurements
- RGB or RGB-D video
- Spill events
- Run timing
- Material and vessel identifiers

## Initial protocol

1. Confirm the target scale is stable.
2. Move the source vessel to the pour pose.
3. Execute a controlled rotation.
4. Hold and return upright.
5. Wait for the scale to settle.
6. Record transferred mass and failures.

## Simulation model

The v0.2 world uses MuJoCo for rigid-body poses, contacts, and deterministic stepping. A separate seeded hybrid process model represents bulk flow rate, discharge delay, retained mass, scale noise, scale settling, pour onset, and spill onset. Keeping these dynamics explicit avoids pretending that a rigid-body engine natively provides scientifically adequate granular flow.

Run the checked-in simulation with:

```bash
cargo run -p worldctl -- simulate-mujoco \
  --world worlds/granular-dosing-v0/world.toml \
  --seed 9 \
  --output runs/granular-seed-9
```

The output directory is immutable: the command fails if it already exists. A completed directory contains the typed outcome, deterministic verifier report, and a content-addressed provenance manifest.

## Calibration loop

The first calibration contract accepts physical trials from exactly one material batch and source-world revision. Each trial records the physical run ID, immutable run-manifest digest, pour hold duration, tilt angle, settled transferred mass, scale-settling status, and typed runtime events.

The current estimator deliberately handles only clean, unsaturated pours at one fixed tilt below the spill threshold. Eventful runs are rejected rather than silently removed; they belong in a later failure-model calibration. At least four observations with varied hold durations are required to identify effective flow rate and discharge delay.

The repository includes a dataset marked `synthetic-calibration-batch-do-not-treat-as-physical-evidence`. It tests the software pipeline only. Run the complete synthetic calibration and replay with:

```bash
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
```

Replacing the synthetic JSON with observations derived from real physical run manifests creates the first genuine real-to-sim update. Held-out physical runs—not the calibration trials—must then evaluate whether the updated world improves prediction.

## Evaluation

The first report must compare calibrated simulation, uncalibrated simulation, and a simple empirical baseline on held-out physical runs. Report mass error, pass/fail discrimination, prediction-interval coverage, spill detection, and runtime reproducibility. Thresholds will be fixed before collecting held-out runs and recorded in a later benchmark revision.

## Explicit exclusions

- Hazardous or respirable powders
- Universal granular or fluid simulation
- Reinforcement-learning policy training
- Photorealistic reconstruction
- Multi-robot orchestration

These exclusions isolate whether the real-to-sim calibration loop works before introducing additional failure sources.
