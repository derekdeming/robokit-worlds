#![doc = "Integration tests for explicit runtime capability negotiation."]

use runtime_api::{
    CapabilityRequirement, EventCapability, MaterialCapability, ProtocolActionCapability,
    RuntimeCapabilities, RuntimeDescriptor, RuntimeKind,
};
use world_domain::{ParameterProperty, WorldDefinition};

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

fn complete_capabilities() -> RuntimeCapabilities {
    RuntimeCapabilities {
        descriptor: RuntimeDescriptor {
            adapter_name: "complete-runtime".to_owned(),
            adapter_version: "0.1.0".to_owned(),
            engine_name: "test-engine".to_owned(),
            engine_version: "0.1.0".to_owned(),
        },
        runtime_kind: RuntimeKind::Simulation,
        protocol_actions: vec![
            ProtocolActionCapability::MoveToPose,
            ProtocolActionCapability::RotateAboutAxis,
            ProtocolActionCapability::Wait,
            ProtocolActionCapability::MeasureMass,
        ],
        materials: vec![MaterialCapability::Granular],
        parameter_properties: vec![
            ParameterProperty::FlowRate,
            ParameterProperty::DischargeDelay,
            ParameterProperty::ResidualMass,
            ParameterProperty::ScaleNoiseStandardDeviation,
            ParameterProperty::ScaleSettlingTime,
            ParameterProperty::InitialMaterialMass,
            ParameterProperty::SpillTiltThreshold,
            ParameterProperty::PourOnsetTiltThreshold,
        ],
        events: vec![
            EventCapability::Spill,
            EventCapability::Collision,
            EventCapability::SafetyStop,
        ],
    }
}

#[test]
fn complete_runtime_accepts_the_benchmark_world() {
    assert!(
        complete_capabilities()
            .supports_world(&granular_world())
            .is_ok()
    );
}

#[test]
fn runtime_never_silently_downgrades_missing_material_behavior() {
    let mut capabilities = complete_capabilities();
    capabilities.materials.clear();

    let error = capabilities
        .supports_world(&granular_world())
        .expect_err("missing granular support must be rejected");
    assert_eq!(
        error.requirements(),
        [CapabilityRequirement::Material(
            MaterialCapability::Granular
        )]
    );
}

#[test]
fn runtime_rejects_an_undetectable_verifier_event() {
    let mut capabilities = complete_capabilities();
    capabilities
        .events
        .retain(|event| *event != EventCapability::SafetyStop);

    let error = capabilities
        .supports_world(&granular_world())
        .expect_err("missing safety-stop detection must be rejected");
    assert_eq!(
        error.requirements(),
        [CapabilityRequirement::Event(EventCapability::SafetyStop)]
    );
}

#[test]
fn runtime_rejects_an_unmodeled_parameter_property() {
    let mut capabilities = complete_capabilities();
    capabilities
        .parameter_properties
        .retain(|property| *property != ParameterProperty::FlowRate);

    let error = capabilities
        .supports_world(&granular_world())
        .expect_err("unmodeled flow-rate uncertainty must be rejected");
    assert_eq!(
        error.requirements(),
        [CapabilityRequirement::Parameter(
            ParameterProperty::FlowRate
        )]
    );
}
