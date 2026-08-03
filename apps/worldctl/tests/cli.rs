#![doc = "End-to-end tests for the worldctl contract boundary."]

use std::{path::PathBuf, process::Command};

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../worlds/granular-dosing-v0")
        .join(path)
}

#[test]
fn validates_world_and_content_addressed_asset() {
    let output = Command::new(env!("CARGO_BIN_EXE_worldctl"))
        .args(["validate-world", "--world"])
        .arg(fixture("world.toml"))
        .output()
        .expect("worldctl should execute");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid world"));
}

#[test]
fn evaluates_the_passing_fixture() {
    let output = Command::new(env!("CARGO_BIN_EXE_worldctl"))
        .args(["evaluate", "--world"])
        .arg(fixture("world.toml"))
        .args(["--outcome"])
        .arg(fixture("passing-outcome.json"))
        .output()
        .expect("worldctl should execute");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"passed\": true"));
}

#[test]
fn validates_manifest_and_content_addressed_outcome() {
    let output = Command::new(env!("CARGO_BIN_EXE_worldctl"))
        .args(["validate-manifest", "--manifest"])
        .arg(fixture("example-run-manifest.json"))
        .output()
        .expect("worldctl should execute");

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("valid run manifest"));
}
