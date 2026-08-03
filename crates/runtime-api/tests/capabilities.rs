#![doc = "Integration tests for explicit runtime capability negotiation."]

use runtime_api::{MaterialCapability, ProtocolActionCapability, RuntimeCapabilities, RuntimeKind};
use world_domain::WorldDefinition;

fn granular_world() -> world_domain::ValidatedWorldDefinition {
    let definition: WorldDefinition = toml::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../worlds/granular-dosing-v0/world.toml"
    )))
    .expect("fixture world should deserialize");
    definition
        .validate()
        .expect("fixture world should validate")
}

#[test]
fn complete_runtime_accepts_the_benchmark_world() {
    let capabilities = RuntimeCapabilities {
        adapter_name: "complete-runtime".to_owned(),
        runtime_kind: RuntimeKind::Simulation,
        protocol_actions: vec![
            ProtocolActionCapability::MoveToPose,
            ProtocolActionCapability::RotateAboutAxis,
            ProtocolActionCapability::Wait,
            ProtocolActionCapability::MeasureMass,
        ],
        materials: vec![MaterialCapability::Granular],
    };

    assert!(capabilities.supports_world(&granular_world()).is_ok());
}

#[test]
fn runtime_never_silently_downgrades_missing_material_behavior() {
    let capabilities = RuntimeCapabilities {
        adapter_name: "rigid-body-only".to_owned(),
        runtime_kind: RuntimeKind::Simulation,
        protocol_actions: vec![
            ProtocolActionCapability::MoveToPose,
            ProtocolActionCapability::RotateAboutAxis,
            ProtocolActionCapability::Wait,
            ProtocolActionCapability::MeasureMass,
        ],
        materials: vec![],
    };

    let error = capabilities
        .supports_world(&granular_world())
        .expect_err("missing granular support must be rejected");
    assert_eq!(error.requirements(), ["material:Granular"]);
}
