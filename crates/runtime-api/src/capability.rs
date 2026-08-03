use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use world_domain::{
    EntityKind, EventKind, MaterialClass, ParameterProperty, ProtocolAction,
    ValidatedWorldDefinition, VerifierDefinition, WorldDefinition,
};

/// Whether a runtime executes a simulator or physical system.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeKind {
    /// Deterministic or stochastic simulator.
    Simulation,
    /// Physical robot and instrument cell.
    Physical,
}

/// Protocol action supported by a runtime.
#[derive(
    Clone, Copy, Debug, Deserialize, Hash, JsonSchema, Ord, PartialEq, Eq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolActionCapability {
    /// World-frame pose motion.
    MoveToPose,
    /// Principal-axis rotation.
    RotateAboutAxis,
    /// Timed wait.
    Wait,
    /// Gravimetric measurement.
    MeasureMass,
}

impl From<&ProtocolAction> for ProtocolActionCapability {
    fn from(action: &ProtocolAction) -> Self {
        match action {
            ProtocolAction::MoveToPose { .. } => Self::MoveToPose,
            ProtocolAction::RotateAboutAxis { .. } => Self::RotateAboutAxis,
            ProtocolAction::Wait { .. } => Self::Wait,
            ProtocolAction::MeasureMass { .. } => Self::MeasureMass,
        }
    }
}

/// Material behavior supported by a runtime.
#[derive(
    Clone, Copy, Debug, Deserialize, Hash, JsonSchema, Ord, PartialEq, Eq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum MaterialCapability {
    /// Discrete or hybrid granular behavior.
    Granular,
}

/// Runtime event that an adapter can detect and report.
#[derive(
    Clone, Copy, Debug, Deserialize, Hash, JsonSchema, Ord, PartialEq, Eq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum EventCapability {
    /// Material spill detection.
    Spill,
    /// Unexpected contact detection.
    Collision,
    /// Safety-stop reporting for terminated physical execution.
    SafetyStop,
    /// Unsettled instrument reading detection.
    UnstableMeasurement,
}

impl From<EventKind> for EventCapability {
    fn from(event: EventKind) -> Self {
        match event {
            EventKind::Spill => Self::Spill,
            EventKind::Collision => Self::Collision,
            EventKind::SafetyStop => Self::SafetyStop,
            EventKind::UnstableMeasurement => Self::UnstableMeasurement,
        }
    }
}

/// Versioned identity of an adapter and the engine it controls.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct RuntimeDescriptor {
    /// Stable adapter implementation name.
    pub adapter_name: String,
    /// Semantic adapter version.
    pub adapter_version: String,
    /// Simulator or physical controller name.
    pub engine_name: String,
    /// Semantic engine version.
    pub engine_version: String,
}

/// Declared runtime behavior used to reject unsupported worlds before execution.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct RuntimeCapabilities {
    /// Versioned runtime identity.
    pub descriptor: RuntimeDescriptor,
    /// Physical or simulated implementation.
    pub runtime_kind: RuntimeKind,
    /// Supported protocol actions.
    pub protocol_actions: Vec<ProtocolActionCapability>,
    /// Supported material behavior classes.
    pub materials: Vec<MaterialCapability>,
    /// Parameter properties this implementation consumes during execution.
    pub parameter_properties: Vec<ParameterProperty>,
    /// Detectable scientific, operational, and safety events.
    pub events: Vec<EventCapability>,
}

impl RuntimeCapabilities {
    /// Checks that every world requirement is explicitly supported.
    ///
    /// # Errors
    ///
    /// Returns all unsupported requirements.
    pub fn supports_world(
        &self,
        world: &ValidatedWorldDefinition,
    ) -> Result<(), CapabilityMismatch> {
        let world = world.definition();
        let supported_actions: HashSet<_> = self.protocol_actions.iter().copied().collect();
        let supported_materials: HashSet<_> = self.materials.iter().copied().collect();
        let supported_parameters: HashSet<_> = self.parameter_properties.iter().copied().collect();
        let supported_events: HashSet<_> = self.events.iter().copied().collect();
        let mut requirements = Vec::new();

        for action in &world.task.steps {
            let required = ProtocolActionCapability::from(&action.action);
            if !supported_actions.contains(&required) {
                requirements.push(CapabilityRequirement::ProtocolAction(required));
            }
        }
        validate_materials(world, &supported_materials, &mut requirements);
        validate_parameters(world, &supported_parameters, &mut requirements);
        validate_events(world, &supported_events, &mut requirements);
        requirements.sort_unstable();
        requirements.dedup();

        if requirements.is_empty() {
            Ok(())
        } else {
            Err(CapabilityMismatch { requirements })
        }
    }
}

fn validate_parameters(
    world: &WorldDefinition,
    supported: &HashSet<ParameterProperty>,
    missing: &mut Vec<CapabilityRequirement>,
) {
    for parameter in &world.parameters {
        let required = parameter.target.property;
        if !supported.contains(&required) {
            missing.push(CapabilityRequirement::Parameter(required));
        }
    }
}

fn validate_materials(
    world: &WorldDefinition,
    supported: &HashSet<MaterialCapability>,
    missing: &mut Vec<CapabilityRequirement>,
) {
    for entity in &world.entities {
        if let EntityKind::Material { class } = entity.entity {
            let required = match class {
                MaterialClass::Granular => MaterialCapability::Granular,
            };
            if !supported.contains(&required) {
                missing.push(CapabilityRequirement::Material(required));
            }
        }
    }
}

fn validate_events(
    world: &WorldDefinition,
    supported: &HashSet<EventCapability>,
    missing: &mut Vec<CapabilityRequirement>,
) {
    for verifier in &world.verifiers {
        if let VerifierDefinition::ForbiddenEventAbsent { event } = verifier {
            let required = EventCapability::from(*event);
            if !supported.contains(&required) {
                missing.push(CapabilityRequirement::Event(required));
            }
        }
    }
}

/// Typed world capability required before execution.
#[derive(
    Clone, Copy, Debug, Deserialize, Hash, JsonSchema, Ord, PartialEq, Eq, PartialOrd, Serialize,
)]
#[serde(tag = "requirement", content = "capability", rename_all = "snake_case")]
pub enum CapabilityRequirement {
    /// Required protocol action.
    ProtocolAction(ProtocolActionCapability),
    /// Required material behavior.
    Material(MaterialCapability),
    /// Required uncertain parameter behavior.
    Parameter(ParameterProperty),
    /// Required event detection.
    Event(EventCapability),
}

/// One or more world requirements unsupported by a runtime.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("runtime lacks required capabilities: {requirements:?}")]
pub struct CapabilityMismatch {
    requirements: Vec<CapabilityRequirement>,
}

impl CapabilityMismatch {
    /// Returns stable textual requirement identifiers.
    pub fn requirements(&self) -> &[CapabilityRequirement] {
        &self.requirements
    }
}
