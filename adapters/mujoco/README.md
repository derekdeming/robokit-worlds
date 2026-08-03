# MuJoCo adapter

This directory owns conversion from a validated RoboKit world into MJCF/MuJoCo runtime state and execution through the `runtime-api` capability boundary.

The adapter must:

- declare exactly which protocol actions, sensors, and parameter targets it supports;
- reject unsupported world requirements before execution;
- preserve the run seed and execution bounds;
- emit typed runtime observations and content-addressed artifacts;
- keep MuJoCo-specific types and parameter names out of `world-domain`.

Implementation begins after the Granular Dosing v0 world contract is accepted. The first implementation will use the official Python bindings for numerical iteration while preserving the Rust-owned execution contract.
