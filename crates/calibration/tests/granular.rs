#![doc = "Integration tests for physical granular calibration."]

use calibration::{CalibrationError, GranularCalibrationDataset, calibrate_granular_world};
use world_domain::{
    EventKind, ParameterDistribution, ParameterProperty, Sha256Digest, WorldDefinition,
};

const SOURCE_WORLD_DIGEST: &str =
    "c076011aeb745e8d2335094d8389a3c37ceefc7ca05b13d32d60b52d0c8bfa4b";

fn source_world() -> world_domain::ValidatedWorldDefinition {
    toml::from_str::<WorldDefinition>(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../worlds/granular-dosing-v0/world.toml"
    )))
    .expect("source world should deserialize")
    .validate()
    .expect("source world should validate")
}

fn dataset() -> GranularCalibrationDataset {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../worlds/granular-dosing-v0/calibration/synthetic-physical-dataset.json"
    )))
    .expect("calibration dataset should deserialize")
}

#[test]
fn calibrated_world_recovers_flow_and_delay() {
    let world = source_world();
    let dataset = dataset()
        .validate_for_world(&world, &Sha256Digest::new(SOURCE_WORLD_DIGEST))
        .expect("dataset should validate");

    let calibrated =
        calibrate_granular_world(&world, &dataset, "0.3.0").expect("calibration should succeed");

    assert!((calibrated.fit().flow_rate.point_estimate - 55.1).abs() < 0.2);
    assert!((calibrated.fit().discharge_delay.point_estimate - 0.18).abs() < 0.01);
    assert!(calibrated.fit().coefficient_of_determination > 0.999);
    let flow = calibrated
        .world()
        .parameters_with_property(ParameterProperty::FlowRate)
        .next()
        .expect("flow parameter should exist");
    assert!(matches!(
        flow.distribution,
        ParameterDistribution::Uniform { .. }
    ));
}

#[test]
fn eventful_trials_are_rejected_instead_of_silently_filtered() {
    let world = source_world();
    let mut dataset = dataset();
    dataset.observations[0].events.push(EventKind::Spill);

    let error = dataset
        .validate_for_world(&world, &Sha256Digest::new(SOURCE_WORLD_DIGEST))
        .expect_err("eventful dataset must be rejected");

    assert!(
        error
            .issues()
            .iter()
            .any(|issue| issue.path == "observations[0].events")
    );
}

#[test]
fn calibrated_version_must_advance() {
    let world = source_world();
    let dataset = dataset()
        .validate_for_world(&world, &Sha256Digest::new(SOURCE_WORLD_DIGEST))
        .expect("dataset should validate");

    assert_eq!(
        calibrate_granular_world(&world, &dataset, "0.2.0"),
        Err(CalibrationError::InvalidWorldVersion)
    );
}
