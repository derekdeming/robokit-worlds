# MuJoCo adapter

This directory owns conversion from a validated RoboKit world into MJCF/MuJoCo runtime state and execution through the `runtime-api` capability boundary.

The adapter must:

- declare exactly which protocol actions, sensors, and parameter targets it supports;
- reject unsupported world requirements before execution;
- preserve the run seed and execution bounds;
- emit typed runtime observations and content-addressed artifacts;
- keep MuJoCo-specific types and parameter names out of `world-domain`.

The first implementation uses the official pinned Python bindings for numerical iteration while preserving a Rust-owned execution contract. Rust owns capability checks, parameter compilation, process supervision, response validation, evaluation, and evidence. Python owns only MuJoCo stepping, contact detection, pose interpolation, and the seeded granular-process state.

The private worker contract is versioned and file-backed. Requests and responses are size-bounded, execution is bounded by both step count and a monotonic deadline, and the worker must report the exact engine version expected by Rust.
