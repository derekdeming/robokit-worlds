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

## Evaluation

The first report must compare calibrated simulation, uncalibrated simulation, and a simple empirical baseline on held-out physical runs. Report mass error, pass/fail discrimination, prediction-interval coverage, spill detection, and runtime reproducibility. Thresholds will be fixed before collecting held-out runs and recorded in a later benchmark revision.

## Explicit exclusions

- Hazardous or respirable powders
- Universal granular or fluid simulation
- Reinforcement-learning policy training
- Photorealistic reconstruction
- Multi-robot orchestration

These exclusions isolate whether the real-to-sim calibration loop works before introducing additional failure sources.
