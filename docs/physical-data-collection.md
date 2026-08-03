# Physical data collection for Granular Dosing v0

This protocol produces the first real calibration dataset. The checked-in synthetic fixture is not a substitute for these runs.

## Experimental controls

- Use one nonhazardous material batch and record its stable batch identifier.
- Keep the robot, source vessel, target vessel, scale, geometry, and source-world revision fixed.
- Use one tilt angle above the world pour-onset bound and below its spill threshold.
- Randomize at least five hold durations across the intended operating range.
- Collect calibration and held-out evaluation runs separately before fitting.
- Start with enough source material that no calibration run approaches retained-mass saturation.

## Required evidence per run

- immutable physical `RunManifest` and its SHA-256 digest;
- unique `run_id`;
- commanded hold duration and tilt angle;
- settled transferred-mass reading;
- scale-settling status;
- typed spill, collision, safety-stop, and unstable-measurement events;
- exact source-world ID, version, and definition digest.

Do not include failed, eventful, unsettled, saturated, or manually corrected trials in the clean-flow fit. Preserve them as immutable physical evidence for the later failure-model dataset rather than deleting or silently filtering them.

## Dataset handoff

Encode accepted calibration trials with the `GranularCalibrationDataset` JSON schema:

```bash
cargo run -p worldctl -- schema \
  --kind calibration-dataset \
  --output /tmp/granular-calibration-dataset.schema.json
```

Then publish and independently reproduce the calibrated world:

```bash
cargo run -p worldctl -- calibrate-granular \
  --world worlds/granular-dosing-v0/world.toml \
  --dataset /path/to/physical-calibration-dataset.json \
  --new-version 0.3.0 \
  --output /path/to/calibrated-world

cargo run -p worldctl -- validate-calibration-package \
  --package /path/to/calibrated-world
```

Finally, execute the calibrated world against held-out conditions and compare its prediction errors with the uncalibrated v0.2 world. Reusing calibration trials as evaluation data does not establish real-to-sim generalization.
