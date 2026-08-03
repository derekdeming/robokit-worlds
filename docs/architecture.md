# Architecture

## Objective

RoboKit Worlds must represent scientific workflows without coupling scientific meaning to a particular simulator, robot, transport, database, or cloud. The core artifact is an immutable, versioned world definition. Runtime adapters compile and execute that definition. Evidence manifests bind every result to the exact world, runtime, inputs, and recorded artifacts that produced it.

## Dependency direction

```text
worldctl ─┬─> MuJoCo / ROS 2 adapter ─> runtime-api ─┐
          ├─> evidence ──────────────────────────────┤
          ├─> evaluation ────────────────────────────┼─> world-domain
          └─> calibration ───────────────────────────┘
```

`world-domain` contains no framework types. It owns identifiers, units, entities, protocols, uncertain parameters, verifiers, and validation. Outer crates may depend inward; the domain never depends outward.

## Contracts

### World definition

A world definition describes:

- semantic entities such as robots, vessels, instruments, materials, and work surfaces;
- a typed protocol graph;
- uncertain physics, process, and sensor parameters;
- quantitative verifiers;
- content-addressed runtime assets.

Deserialization is not validation. Callers must obtain a `ValidatedWorldDefinition` before execution or evaluation.

### Runtime boundary

`runtime-api` defines bounded, one-run execution. Every request supplies a wall-clock timeout and maximum step count. Capabilities are typed across protocol actions, material behavior, consumed parameter properties, and detectable events, so a runtime rejects an unsupported world before it starts. The initial API intentionally models scripted protocols; interactive policy stepping will be introduced as a separate capability once the first closed-loop benchmark proves the execution semantics.

The MuJoCo adapter preserves a strict ownership split. Rust validates the canonical world, compiles the engine-neutral protocol and bounded process parameters, supervises the subprocess, checks the response, and publishes evidence. The pinned Python worker loads MJCF, advances MuJoCo, and co-simulates the granular-process state. It cannot parse a world definition, evaluate success, publish evidence, or choose an alternate behavior.

The worker request is file-backed and size-bounded. The process has a monotonic deadline, bounded step count, null stdin, bounded diagnostics, exact MuJoCo version validation, and startup grace owned by the Rust supervisor. Successful responses are written atomically.

Distributed execution uses at-least-once delivery. `run_id` is the idempotency key across deliveries, while the one-based attempt number identifies a worker lease attempt. A durable orchestrator must allow only one committed result per run, reject conflicting world revisions, bound lease duration, and route exhausted attempts to a terminal failure state. Runtime adapters accept only validated requests, so unbounded work cannot cross the execution boundary.

### Evidence

Run manifests are immutable provenance records. They include exact adapter and engine versions. Large telemetry and media remain external and are referenced by SHA-256 digest. MCAP is the canonical time-series container for physical robot and sensor data. Local execution publishes a run directory only after the outcome, evaluation, and manifest are complete.

### Evaluation

Evaluation consumes a validated world plus a typed run outcome. Verifier behavior has one implementation in the `evaluation` crate and is shared across simulator and physical runs.

### Calibration

`calibration` consumes a validated source world plus a versioned, batch-homogeneous physical-observation dataset. Every observation carries a unique physical run ID and the SHA-256 digest of its immutable run manifest. The initial granular estimator rejects unsettled readings, eventful runs, saturation, spill-regime tilt, mismatched world revisions, and insufficient hold-duration variation.

The v1 estimator fits effective flow rate and discharge delay with a two-parameter linear model, estimates scale noise from residuals, and writes two-standard-error bounded distributions into a strictly newer world revision. The estimator never mutates the source package. `worldctl` atomically publishes a self-contained calibrated package containing the exact source world bytes, source dataset bytes, copied content-addressed assets, calibrated world, and immutable calibration report.

Package validation re-hashes every source, validates the calibrated world assets, and deterministically recomputes both the fit and calibrated definition. This makes calibration a reproducible scientific transformation rather than an opaque model update.

## Initial deployment boundary

The first milestone remains local-first:

- macOS and Linux development with pinned MuJoCo execution;
- Ubuntu 24.04 with ROS 2 Jazzy for physical hardware;
- file-backed world packages and immutable run bundles;
- no database, distributed queue, web service, or UI until the benchmark establishes the required access patterns.

This avoids premature infrastructure without constraining later distributed execution. Runtime requests already define explicit bounds, identifiers, and artifact contracts suitable for a queued worker architecture.
