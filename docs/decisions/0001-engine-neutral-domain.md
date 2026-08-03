# ADR 0001: Keep scientific-world semantics engine-neutral

- Status: accepted
- Date: 2026-08-03

## Context

MuJoCo is the fastest path to the first local benchmark, while Isaac Sim and physical ROS 2 cells are expected future runtimes. Making MJCF, USD, ROS messages, or simulator-specific parameters the canonical product model would move scientific meaning into an adapter and make evidence incomparable across runtimes.

## Decision

The canonical world model is a typed Rust domain independent of simulator and robot frameworks. Runtime assets are content-addressed references. Adapters compile the canonical model into their native format and declare supported capabilities before execution.

## Consequences

- Scientific-world validation and verifier logic have one owner.
- Adapter implementations must translate explicit domain concepts.
- A world can be rejected when a runtime lacks required capabilities; there is no silent downgrade.
- The initial schema remains deliberately narrow and grows only from demonstrated benchmark requirements.
