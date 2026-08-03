use std::path::Path;

use runtime_api::ValidatedExecutionRequest;
use world_domain::{
    AssetFormat, EntityKind, EventKind, ParameterDistribution, ParameterProperty, ProtocolAction,
    ValidatedWorldDefinition, VerifierDefinition,
};

use crate::{
    MujocoAdapterError,
    binding::MujocoBinding,
    contract::{AdapterContractSchemaVersion, AdapterExecutionRequest, GranularProcessSpec},
};

pub(crate) fn compile_request(
    world: &ValidatedWorldDefinition,
    request: &ValidatedExecutionRequest,
    binding: &MujocoBinding,
    package_root: &Path,
) -> Result<AdapterExecutionRequest, MujocoAdapterError> {
    let definition = world.definition();
    let controlled_entity = definition
        .entities
        .iter()
        .find(|entity| entity.id == binding.controlled_entity_id)
        .ok_or_else(|| {
            MujocoAdapterError::InvalidBinding(format!(
                "controlled entity `{}` does not exist",
                binding.controlled_entity_id
            ))
        })?;
    if !matches!(controlled_entity.entity, EntityKind::Robot { .. }) {
        return Err(MujocoAdapterError::InvalidBinding(format!(
            "controlled entity `{}` is not a robot",
            binding.controlled_entity_id
        )));
    }

    let scene_declared = definition.entities.iter().any(|entity| {
        entity.asset.as_ref().is_some_and(|asset| {
            asset.logical_path == binding.scene_asset && asset.format == AssetFormat::Mjcf
        })
    });
    if !scene_declared {
        return Err(MujocoAdapterError::MissingSceneAsset(
            binding.scene_asset.to_string(),
        ));
    }

    validate_protocol_actors(world, binding)?;
    validate_event_bindings(world, binding)?;
    let process = GranularProcessSpec {
        flow_rate: required_positive_distribution(world, ParameterProperty::FlowRate)?,
        discharge_delay: required_non_negative_distribution(
            world,
            ParameterProperty::DischargeDelay,
        )?,
        residual_mass: required_non_negative_distribution(world, ParameterProperty::ResidualMass)?,
        scale_noise_standard_deviation: required_non_negative_distribution(
            world,
            ParameterProperty::ScaleNoiseStandardDeviation,
        )?,
        scale_settling_time: required_non_negative_distribution(
            world,
            ParameterProperty::ScaleSettlingTime,
        )?,
        initial_material_mass: required_positive_distribution(
            world,
            ParameterProperty::InitialMaterialMass,
        )?,
        spill_tilt_threshold: required_positive_distribution(
            world,
            ParameterProperty::SpillTiltThreshold,
        )?,
        pour_onset_tilt_threshold: required_non_negative_distribution(
            world,
            ParameterProperty::PourOnsetTiltThreshold,
        )?,
    };
    validate_process_relationships(&process)?;

    let request = request.request();
    Ok(AdapterExecutionRequest {
        schema_version: AdapterContractSchemaVersion::V1,
        run_id: request.run_id,
        attempt: request.attempt,
        seed: request.seed,
        limits: request.limits,
        scene_path: package_root.join(binding.scene_asset.as_str()),
        controlled_body: binding.controlled_body.clone(),
        forbidden_contact_geometries: binding.forbidden_contact_geometries.clone(),
        protocol: definition.task.steps.clone(),
        process,
    })
}

fn validate_event_bindings(
    world: &ValidatedWorldDefinition,
    binding: &MujocoBinding,
) -> Result<(), MujocoAdapterError> {
    let collision_detection_required = world.definition().verifiers.iter().any(|verifier| {
        matches!(
            verifier,
            VerifierDefinition::ForbiddenEventAbsent {
                event: EventKind::Collision
            }
        )
    });
    if collision_detection_required && binding.forbidden_contact_geometries.is_empty() {
        return Err(MujocoAdapterError::InvalidBinding(
            "collision verification requires at least one forbidden contact geometry".to_owned(),
        ));
    }
    Ok(())
}

fn validate_protocol_actors(
    world: &ValidatedWorldDefinition,
    binding: &MujocoBinding,
) -> Result<(), MujocoAdapterError> {
    for step in &world.definition().task.steps {
        let actor_id = match &step.action {
            ProtocolAction::MoveToPose { actor_id, .. }
            | ProtocolAction::RotateAboutAxis { actor_id, .. } => Some(actor_id),
            ProtocolAction::Wait { .. } | ProtocolAction::MeasureMass { .. } => None,
        };
        if let Some(actor_id) = actor_id
            && actor_id != &binding.controlled_entity_id
        {
            return Err(MujocoAdapterError::WrongControlledEntity {
                step_id: step.id.to_string(),
                expected: binding.controlled_entity_id.to_string(),
                actual: actor_id.to_string(),
            });
        }
    }
    Ok(())
}

fn required_positive_distribution(
    world: &ValidatedWorldDefinition,
    property: ParameterProperty,
) -> Result<ParameterDistribution, MujocoAdapterError> {
    let distribution = required_distribution(world, property)?;
    if distribution_has_strictly_positive_support(distribution) {
        Ok(distribution)
    } else {
        Err(MujocoAdapterError::InvalidParameter {
            property,
            reason: "distribution must have strictly positive bounded support".to_owned(),
        })
    }
}

fn required_non_negative_distribution(
    world: &ValidatedWorldDefinition,
    property: ParameterProperty,
) -> Result<ParameterDistribution, MujocoAdapterError> {
    let distribution = required_distribution(world, property)?;
    if distribution_has_non_negative_support(distribution) {
        Ok(distribution)
    } else {
        Err(MujocoAdapterError::InvalidParameter {
            property,
            reason: "distribution must have non-negative bounded support".to_owned(),
        })
    }
}

fn required_distribution(
    world: &ValidatedWorldDefinition,
    property: ParameterProperty,
) -> Result<ParameterDistribution, MujocoAdapterError> {
    let mut matches = world
        .parameters_with_property(property)
        .map(|parameter| parameter.distribution);
    let distribution = matches
        .next()
        .ok_or(MujocoAdapterError::MissingParameter(property))?;
    if matches.next().is_some() {
        return Err(MujocoAdapterError::DuplicateParameter(property));
    }
    Ok(distribution)
}

fn distribution_has_strictly_positive_support(distribution: ParameterDistribution) -> bool {
    match distribution {
        ParameterDistribution::Fixed { value } => value > 0.0,
        ParameterDistribution::Uniform { minimum, .. } => minimum > 0.0,
        ParameterDistribution::Normal { .. } => false,
    }
}

fn distribution_has_non_negative_support(distribution: ParameterDistribution) -> bool {
    match distribution {
        ParameterDistribution::Fixed { value } => value >= 0.0,
        ParameterDistribution::Uniform { minimum, .. } => minimum >= 0.0,
        ParameterDistribution::Normal { .. } => false,
    }
}

fn validate_process_relationships(process: &GranularProcessSpec) -> Result<(), MujocoAdapterError> {
    let onset_maximum = distribution_maximum(process.pour_onset_tilt_threshold);
    let spill_minimum = distribution_minimum(process.spill_tilt_threshold);
    if onset_maximum >= spill_minimum {
        return Err(MujocoAdapterError::InvalidParameter {
            property: ParameterProperty::SpillTiltThreshold,
            reason: "spill threshold must exceed every pour-onset value".to_owned(),
        });
    }
    Ok(())
}

fn distribution_minimum(distribution: ParameterDistribution) -> f64 {
    match distribution {
        ParameterDistribution::Fixed { value } => value,
        ParameterDistribution::Uniform { minimum, .. } => minimum,
        ParameterDistribution::Normal { .. } => f64::NEG_INFINITY,
    }
}

fn distribution_maximum(distribution: ParameterDistribution) -> f64 {
    match distribution {
        ParameterDistribution::Fixed { value } => value,
        ParameterDistribution::Uniform { maximum, .. } => maximum,
        ParameterDistribution::Normal { .. } => f64::INFINITY,
    }
}
