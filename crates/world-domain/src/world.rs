use std::collections::{HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    EntityId, EntityKind, InstrumentKind, MeasurementId, ParameterDefinition, ProtocolAction,
    ProtocolStep, TaskDefinition, ValidationCode, ValidationIssue, VerifierDefinition, WorldEntity,
    WorldId, WorldValidationErrors, WorldVersion,
};

/// Version of the serialized world-definition schema.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub enum SchemaVersion {
    /// Initial executable-world contract.
    #[serde(rename = "1.0.0")]
    V1,
}

/// Engine-neutral definition of one executable scientific world.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct WorldDefinition {
    /// Serialized-contract version.
    pub schema_version: SchemaVersion,
    /// Stable identity shared across world revisions.
    pub id: WorldId,
    /// Semantic version of this exact world definition.
    pub version: WorldVersion,
    /// Human-readable world name.
    pub name: String,
    /// Semantic entities available to protocols and parameters.
    pub entities: Vec<WorldEntity>,
    /// Ordered scientific protocol.
    pub task: TaskDefinition,
    /// Calibratable world uncertainty.
    pub parameters: Vec<ParameterDefinition>,
    /// Quantitative success and failure criteria.
    pub verifiers: Vec<VerifierDefinition>,
}

impl WorldDefinition {
    /// Validates all cross-field invariants and returns an execution-safe wrapper.
    ///
    /// # Errors
    ///
    /// Returns every detected boundary-validation issue.
    pub fn validate(self) -> Result<ValidatedWorldDefinition, WorldValidationErrors> {
        let issues = self.validation_issues();
        if issues.is_empty() {
            Ok(ValidatedWorldDefinition(self))
        } else {
            Err(WorldValidationErrors::new(issues))
        }
    }

    /// Reports all boundary-validation issues without consuming the definition.
    pub fn validation_issues(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        self.validate_header(&mut issues);
        let entities = self.validate_entities(&mut issues);
        let measurement_ids = self.validate_task(&entities, &mut issues);
        self.validate_parameters(&entities, &mut issues);
        self.validate_verifiers(&measurement_ids, &mut issues);
        issues
    }

    fn validate_header(&self, issues: &mut Vec<ValidationIssue>) {
        if !self.version.is_valid_semver() {
            issues.push(ValidationIssue::new(
                "version",
                ValidationCode::InvalidVersion,
                "world version must be valid semantic versioning",
            ));
        }
        if self.name.trim().is_empty() {
            issues.push(ValidationIssue::new(
                "name",
                ValidationCode::EmptyValue,
                "world name must not be empty",
            ));
        }
        if self.entities.is_empty() {
            issues.push(ValidationIssue::new(
                "entities",
                ValidationCode::EmptyValue,
                "world must contain at least one entity",
            ));
        }
        if self.verifiers.is_empty() {
            issues.push(ValidationIssue::new(
                "verifiers",
                ValidationCode::EmptyValue,
                "world must contain at least one verifier",
            ));
        }
    }

    fn validate_entities(
        &self,
        issues: &mut Vec<ValidationIssue>,
    ) -> HashMap<EntityId, EntityKind> {
        let mut entities = HashMap::new();
        for (index, entity) in self.entities.iter().enumerate() {
            let base = format!("entities[{index}]");
            validate_id(&entity.id, format!("{base}.id"), issues);
            if entities.insert(entity.id.clone(), entity.entity).is_some() {
                issues.push(ValidationIssue::new(
                    format!("{base}.id"),
                    ValidationCode::DuplicateIdentifier,
                    format!("entity identifier `{}` is duplicated", entity.id),
                ));
            }
            if entity.name.trim().is_empty() {
                issues.push(ValidationIssue::new(
                    format!("{base}.name"),
                    ValidationCode::EmptyValue,
                    "entity name must not be empty",
                ));
            }
            if !entity.initial_pose.position.is_finite()
                || !entity.initial_pose.orientation.is_normalized()
            {
                issues.push(ValidationIssue::new(
                    format!("{base}.initial_pose"),
                    ValidationCode::InvalidValue,
                    "pose must be finite and its quaternion must be normalized",
                ));
            }
            if let Some(asset) = &entity.asset {
                if !asset.logical_path.is_well_formed() {
                    issues.push(ValidationIssue::new(
                        format!("{base}.asset.logical_path"),
                        ValidationCode::UnsafeAssetPath,
                        "asset path must remain inside the world package",
                    ));
                }
                if !asset.sha256.is_well_formed() {
                    issues.push(ValidationIssue::new(
                        format!("{base}.asset.sha256"),
                        ValidationCode::InvalidAssetDigest,
                        "asset digest must be 64 lowercase hexadecimal characters",
                    ));
                }
            }
        }
        entities
    }

    fn validate_task(
        &self,
        entities: &HashMap<EntityId, EntityKind>,
        issues: &mut Vec<ValidationIssue>,
    ) -> HashSet<MeasurementId> {
        validate_id(&self.task.id, "task.id", issues);
        if self.task.name.trim().is_empty() {
            issues.push(ValidationIssue::new(
                "task.name",
                ValidationCode::EmptyValue,
                "task name must not be empty",
            ));
        }
        if self.task.steps.is_empty() {
            issues.push(ValidationIssue::new(
                "task.steps",
                ValidationCode::EmptyValue,
                "task must contain at least one protocol step",
            ));
        }

        let mut step_ids = HashSet::new();
        let mut measurement_ids = HashSet::new();
        for (index, step) in self.task.steps.iter().enumerate() {
            let base = format!("task.steps[{index}]");
            validate_step(step, &base, entities, issues);
            if !step_ids.insert(step.id.clone()) {
                issues.push(ValidationIssue::new(
                    format!("{base}.id"),
                    ValidationCode::DuplicateIdentifier,
                    format!("protocol step identifier `{}` is duplicated", step.id),
                ));
            }
            if let Some(measurement_id) = step.action.produced_measurement() {
                validate_id(measurement_id, format!("{base}.measurement_id"), issues);
                if !measurement_ids.insert(measurement_id.clone()) {
                    issues.push(ValidationIssue::new(
                        format!("{base}.measurement_id"),
                        ValidationCode::DuplicateIdentifier,
                        format!("measurement identifier `{measurement_id}` is duplicated"),
                    ));
                }
            }
        }
        measurement_ids
    }

    fn validate_parameters(
        &self,
        entities: &HashMap<EntityId, EntityKind>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        let mut ids = HashSet::new();
        for (index, parameter) in self.parameters.iter().enumerate() {
            let base = format!("parameters[{index}]");
            validate_id(&parameter.id, format!("{base}.id"), issues);
            if !ids.insert(parameter.id.clone()) {
                issues.push(ValidationIssue::new(
                    format!("{base}.id"),
                    ValidationCode::DuplicateIdentifier,
                    format!("parameter identifier `{}` is duplicated", parameter.id),
                ));
            }
            match entities.get(&parameter.target.entity_id) {
                None => issues.push(ValidationIssue::new(
                    format!("{base}.target.entity_id"),
                    ValidationCode::MissingReference,
                    format!(
                        "parameter references unknown entity `{}`",
                        parameter.target.entity_id
                    ),
                )),
                Some(entity) if !parameter.target.property.supports_entity(*entity) => {
                    issues.push(ValidationIssue::new(
                        format!("{base}.target.entity_id"),
                        ValidationCode::InvalidReferenceType,
                        format!(
                            "property `{:?}` is not valid for entity `{}`",
                            parameter.target.property, parameter.target.entity_id
                        ),
                    ));
                }
                Some(_) => {}
            }
            if parameter.unit != parameter.target.property.expected_unit() {
                issues.push(ValidationIssue::new(
                    format!("{base}.unit"),
                    ValidationCode::InvalidValue,
                    format!(
                        "property `{:?}` requires unit `{:?}`",
                        parameter.target.property,
                        parameter.target.property.expected_unit()
                    ),
                ));
            }
            if !parameter.distribution.is_valid() {
                issues.push(ValidationIssue::new(
                    format!("{base}.distribution"),
                    ValidationCode::InvalidValue,
                    "parameter distribution must be finite and have a valid range",
                ));
            }
        }
    }

    fn validate_verifiers(
        &self,
        measurement_ids: &HashSet<MeasurementId>,
        issues: &mut Vec<ValidationIssue>,
    ) {
        for (index, verifier) in self.verifiers.iter().enumerate() {
            let base = format!("verifiers[{index}]");
            match verifier {
                VerifierDefinition::MassWithinTolerance {
                    measurement_id,
                    target,
                    tolerance,
                } => {
                    if !measurement_ids.contains(measurement_id) {
                        issues.push(ValidationIssue::new(
                            format!("{base}.measurement_id"),
                            ValidationCode::MissingReference,
                            format!("verifier references unknown measurement `{measurement_id}`"),
                        ));
                    }
                    if !target.is_non_negative_finite()
                        || !tolerance.is_non_negative_finite()
                        || tolerance.get() == 0.0
                    {
                        issues.push(ValidationIssue::new(
                            base,
                            ValidationCode::InvalidValue,
                            "mass target must be non-negative and tolerance must be positive",
                        ));
                    }
                }
                VerifierDefinition::MaximumDuration { limit } => {
                    if !limit.is_non_negative_finite() || limit.get() == 0.0 {
                        issues.push(ValidationIssue::new(
                            base,
                            ValidationCode::InvalidValue,
                            "maximum duration must be positive and finite",
                        ));
                    }
                }
                VerifierDefinition::ForbiddenEventAbsent { .. } => {}
            }
        }
    }
}

fn validate_step(
    step: &ProtocolStep,
    base: &str,
    entities: &HashMap<EntityId, EntityKind>,
    issues: &mut Vec<ValidationIssue>,
) {
    validate_id(&step.id, format!("{base}.id"), issues);
    for entity_id in step.action.referenced_entities() {
        if !entities.contains_key(entity_id) {
            issues.push(ValidationIssue::new(
                format!("{base}.action"),
                ValidationCode::MissingReference,
                format!("protocol action references unknown entity `{entity_id}`"),
            ));
        }
    }
    validate_action_entity_types(&step.action, base, entities, issues);
    if !step.action.has_valid_values() {
        issues.push(ValidationIssue::new(
            format!("{base}.action"),
            ValidationCode::InvalidValue,
            "protocol action contains a non-finite pose, invalid quaternion, or invalid duration",
        ));
    }
}

fn validate_action_entity_types(
    action: &ProtocolAction,
    base: &str,
    entities: &HashMap<EntityId, EntityKind>,
    issues: &mut Vec<ValidationIssue>,
) {
    let invalid = match action {
        ProtocolAction::MoveToPose { actor_id, .. }
        | ProtocolAction::RotateAboutAxis { actor_id, .. } => entities
            .get(actor_id)
            .is_some_and(|entity| !matches!(entity, EntityKind::Robot { .. })),
        ProtocolAction::MeasureMass { instrument_id, .. } => {
            entities.get(instrument_id).is_some_and(|entity| {
                !matches!(
                    entity,
                    EntityKind::Instrument {
                        instrument: InstrumentKind::Scale
                    }
                )
            })
        }
        ProtocolAction::Wait { .. } => false,
    };
    if invalid {
        issues.push(ValidationIssue::new(
            format!("{base}.action"),
            ValidationCode::InvalidReferenceType,
            "protocol action references an entity without the required capability",
        ));
    }
}

trait ValidatableId {
    fn is_valid_id(&self) -> bool;
}

macro_rules! impl_validatable_id {
    ($($type:ty),+ $(,)?) => {
        $(
            impl ValidatableId for $type {
                fn is_valid_id(&self) -> bool {
                    self.is_valid()
                }
            }
        )+
    };
}

impl_validatable_id!(
    crate::EntityId,
    crate::MeasurementId,
    crate::ParameterId,
    crate::ProtocolStepId,
    crate::TaskId,
);

fn validate_id(
    id: &impl ValidatableId,
    path: impl Into<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    if !id.is_valid_id() {
        issues.push(ValidationIssue::new(
            path,
            ValidationCode::InvalidIdentifier,
            "identifier must contain 1-128 ASCII letters, digits, hyphens, or underscores",
        ));
    }
}

/// World definition whose boundary and cross-reference invariants have been checked.
#[derive(Clone, Debug, PartialEq)]
pub struct ValidatedWorldDefinition(WorldDefinition);

impl ValidatedWorldDefinition {
    /// Borrows the validated definition.
    pub const fn definition(&self) -> &WorldDefinition {
        &self.0
    }

    /// Consumes the wrapper and returns the underlying definition.
    pub fn into_definition(self) -> WorldDefinition {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;
    use crate::{
        Axis, EntityKind, EventKind, Grams, InstrumentKind, MaterialClass, MeasurementId,
        ParameterDistribution, ParameterId, ParameterProperty, ParameterTarget, ParameterUnit,
        Pose, Position3, ProtocolAction, ProtocolStepId, Quaternion, Radians, RobotEmbodiment,
        Seconds, TaskId, Vector3, WorldVersion,
    };

    fn valid_world() -> WorldDefinition {
        let neutral_pose = Pose {
            position: Position3 {
                x: crate::Meters::new(0.0),
                y: crate::Meters::new(0.0),
                z: crate::Meters::new(0.0),
            },
            orientation: Quaternion {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            },
        };
        WorldDefinition {
            schema_version: SchemaVersion::V1,
            id: WorldId::from_uuid(Uuid::nil()),
            version: WorldVersion::new("0.1.0"),
            name: "test world".to_owned(),
            entities: vec![
                WorldEntity {
                    id: EntityId::new("robot"),
                    name: "robot".to_owned(),
                    entity: EntityKind::Robot {
                        embodiment: RobotEmbodiment::Manipulator,
                    },
                    initial_pose: neutral_pose,
                    asset: None,
                },
                WorldEntity {
                    id: EntityId::new("scale"),
                    name: "scale".to_owned(),
                    entity: EntityKind::Instrument {
                        instrument: InstrumentKind::Scale,
                    },
                    initial_pose: neutral_pose,
                    asset: None,
                },
                WorldEntity {
                    id: EntityId::new("material"),
                    name: "material".to_owned(),
                    entity: EntityKind::Material {
                        class: MaterialClass::Granular,
                    },
                    initial_pose: neutral_pose,
                    asset: None,
                },
            ],
            task: TaskDefinition {
                id: TaskId::new("dose"),
                name: "dose".to_owned(),
                steps: vec![
                    ProtocolStep {
                        id: ProtocolStepId::new("pour"),
                        action: ProtocolAction::RotateAboutAxis {
                            actor_id: EntityId::new("robot"),
                            axis: Axis::Y,
                            angle: Radians::new(1.0),
                            duration: Seconds::new(1.0),
                        },
                    },
                    ProtocolStep {
                        id: ProtocolStepId::new("measure"),
                        action: ProtocolAction::MeasureMass {
                            instrument_id: EntityId::new("scale"),
                            measurement_id: MeasurementId::new("transferred_mass"),
                        },
                    },
                ],
            },
            parameters: vec![ParameterDefinition {
                id: ParameterId::new("flow_rate"),
                target: ParameterTarget {
                    entity_id: EntityId::new("material"),
                    property: ParameterProperty::FlowRate,
                },
                unit: ParameterUnit::GramsPerSecond,
                distribution: ParameterDistribution::Uniform {
                    minimum: 1.0,
                    maximum: 2.0,
                },
            }],
            verifiers: vec![
                VerifierDefinition::MassWithinTolerance {
                    measurement_id: MeasurementId::new("transferred_mass"),
                    target: Grams::new(10.0),
                    tolerance: Grams::new(1.0),
                },
                VerifierDefinition::ForbiddenEventAbsent {
                    event: EventKind::Spill,
                },
            ],
        }
    }

    #[test]
    fn valid_world_passes_boundary_validation() {
        assert!(valid_world().validate().is_ok());
    }

    #[test]
    fn validation_aggregates_duplicate_and_missing_references() {
        let mut world = valid_world();
        world.entities[1].id = EntityId::new("robot");
        world.task.steps[0].action = ProtocolAction::RotateAboutAxis {
            actor_id: EntityId::new("missing"),
            axis: Axis::Y,
            angle: Radians::new(1.0),
            duration: Seconds::new(1.0),
        };

        let error = world.validate().expect_err("world should be invalid");
        assert!(
            error
                .issues()
                .iter()
                .any(|issue| issue.code == ValidationCode::DuplicateIdentifier)
        );
        assert!(
            error
                .issues()
                .iter()
                .any(|issue| issue.code == ValidationCode::MissingReference)
        );
    }

    #[test]
    fn vector_finiteness_rejects_nan() {
        assert!(
            !Vector3 {
                x: f64::NAN,
                y: 0.0,
                z: 0.0,
            }
            .is_finite()
        );
    }
}
