# ADR 0002: Reproducible calibration packages

## Status

Accepted

## Context

Real-to-sim calibration changes scientific-world parameters using physical evidence. Treating this as an in-place configuration edit would lose the relationship among the physical runs, estimator, source world, fitted values, and simulator revision.

## Decision

Calibration is an engine-neutral, deterministic transformation owned by the `calibration` crate. It accepts only validated, versioned observation contracts and publishes a strictly newer world revision.

`worldctl` writes a new immutable package atomically. The package contains:

- exact source-world bytes;
- exact source-dataset bytes;
- copied content-addressed runtime assets;
- the calibrated world definition;
- an immutable report with algorithm identity, input/output digests, estimates, uncertainty, and fit metrics.

Validation replays the estimator and requires the fitted report and calibrated definition to match within a strict floating-point serialization tolerance. Source packages are never modified.

## Consequences

Calibration results are independently reproducible and can be compared or rolled back by selecting a world revision. Physical observations must retain immutable run-manifest provenance. New scientific regimes or estimators require explicit contract and method versions rather than hidden behavior changes.
