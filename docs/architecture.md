# Architecture

## Objective

RoboKit Worlds must represent scientific workflows without coupling scientific meaning to a particular simulator, robot, transport, database, or cloud. The core artifact is an immutable, versioned world definition. Runtime adapters compile and execute that definition. Evidence manifests bind every result to the exact world, runtime, inputs, and recorded artifacts that produced it.

## Dependency direction

```text
                         ┌──────────────────┐
                         │   world-domain   │
                         └────────▲─────────┘
                                  │
               ┌──────────────────┼──────────────────┐
               │                  │                  │
       ┌───────┴───────┐  ┌───────┴──────┐  ┌────────┴───────┐
       │  runtime-api  │  │   evidence   │  │   evaluation   │
       └───────▲───────┘  └───────▲──────┘  └────────▲───────┘
               │                  │                  │
       ┌───────┴────────┐         └────────┬─────────┘
       │ runtime adapter│                  │
       │ MuJoCo / ROS 2 │          ┌───────┴───────┐
       └────────────────┘          │    worldctl   │
                                   └───────────────┘
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

`runtime-api` defines bounded, one-run execution. Every request supplies a wall-clock timeout and maximum step count. The initial API intentionally models scripted protocols; interactive policy stepping will be introduced as a separate capability once the first closed-loop benchmark proves the execution semantics.

Distributed execution uses at-least-once delivery. `run_id` is the idempotency key across deliveries, while the one-based attempt number identifies a worker lease attempt. A durable orchestrator must allow only one committed result per run, reject conflicting world revisions, bound lease duration, and route exhausted attempts to a terminal failure state. Runtime adapters accept only validated requests, so unbounded work cannot cross the execution boundary.

### Evidence

Run manifests are immutable provenance records. Large telemetry and media remain external and are referenced by SHA-256 digest. MCAP is the canonical time-series container for physical robot and sensor data.

### Evaluation

Evaluation consumes a validated world plus a typed run outcome. Verifier behavior has one implementation in the `evaluation` crate and is shared across simulator and physical runs.

## Initial deployment boundary

The first milestone is local-first:

- macOS development and MuJoCo execution;
- Ubuntu 24.04 with ROS 2 Jazzy for physical hardware;
- file-backed world packages and immutable run bundles;
- no database, distributed queue, web service, or UI until the benchmark establishes the required access patterns.

This avoids premature infrastructure without constraining later distributed execution. Runtime requests already define explicit bounds, identifiers, and artifact contracts suitable for a queued worker architecture.
