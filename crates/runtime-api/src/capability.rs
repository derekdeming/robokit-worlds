use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use world_domain::{
    EntityKind, MaterialClass, ProtocolAction, ValidatedWorldDefinition, WorldDefinition,
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
#[derive(Clone, Copy, Debug, Deserialize, Hash, JsonSchema, PartialEq, Eq, Serialize)]
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
#[derive(Clone, Copy, Debug, Deserialize, Hash, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialCapability {
    /// Discrete or hybrid granular behavior.
    Granular,
}

/// Declared runtime behavior used to reject unsupported worlds before execution.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct RuntimeCapabilities {
    /// Stable adapter name.
    pub adapter_name: String,
    /// Physical or simulated implementation.
    pub runtime_kind: RuntimeKind,
    /// Supported protocol actions.
    pub protocol_actions: Vec<ProtocolActionCapability>,
    /// Supported material behavior classes.
    pub materials: Vec<MaterialCapability>,
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
        let mut requirements = Vec::new();

        for action in &world.task.steps {
            let required = ProtocolActionCapability::from(&action.action);
            if !supported_actions.contains(&required) {
                requirements.push(format!("protocol_action:{required:?}"));
            }
        }
        validate_materials(world, &supported_materials, &mut requirements);
        requirements.sort_unstable();
        requirements.dedup();

        if requirements.is_empty() {
            Ok(())
        } else {
            Err(CapabilityMismatch { requirements })
        }
    }
}

fn validate_materials(
    world: &WorldDefinition,
    supported: &HashSet<MaterialCapability>,
    missing: &mut Vec<String>,
) {
    for entity in &world.entities {
        if let EntityKind::Material { class } = entity.entity {
            let required = match class {
                MaterialClass::Granular => MaterialCapability::Granular,
            };
            if !supported.contains(&required) {
                missing.push(format!("material:{required:?}"));
            }
        }
    }
}

/// One or more world requirements unsupported by a runtime.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("runtime lacks required capabilities: {requirements:?}")]
pub struct CapabilityMismatch {
    requirements: Vec<String>,
}

impl CapabilityMismatch {
    /// Returns stable textual requirement identifiers.
    pub fn requirements(&self) -> &[String] {
        &self.requirements
    }
}
