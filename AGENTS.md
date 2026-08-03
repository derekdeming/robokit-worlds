# Repository guidance

RoboKit Worlds is a Rust-first executable-world platform. Production changes must preserve the dependency direction documented in `docs/architecture.md`.

## Boundaries

- `world-domain` owns engine-neutral scientific-world concepts and validation.
- `runtime-api` owns the capability boundary implemented by simulator and robot adapters.
- `evidence` owns immutable run provenance and artifact references.
- `evaluation` owns deterministic verifier execution.
- Applications compose crates; they do not own business rules.
- Simulator- and ROS-specific types must remain under `adapters/`.

Do not introduce compatibility paths, silent fallbacks, untyped property maps, or duplicated verifier logic. Treat serialized world definitions and run manifests as versioned contracts. Keep files below 1,200 lines and split by ownership before they approach that limit.

## Validation

Run focused checks for touched packages during development. Before publishing a cohesive workspace-wide change, run:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```
