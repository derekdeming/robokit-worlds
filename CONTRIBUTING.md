# Contributing

## Change shape

Keep each change cohesive around one behavioral goal. Read the owning crate before editing it, preserve dependency direction, and add abstractions only when they remove demonstrated complexity.

Serialized contracts require explicit schema-version consideration. Do not silently reinterpret existing fields or add compatibility shims. During this development phase, make clean contract cutovers and update fixtures atomically.

## Checks

Use focused package checks while iterating:

```bash
cargo test -p world-domain
cargo test -p evaluation
```

Before publishing a workspace-wide change:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
```
