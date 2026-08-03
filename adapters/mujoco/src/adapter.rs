use std::path::{Path, PathBuf};

use runtime_api::{
    EventCapability, MaterialCapability, ProtocolActionCapability, RuntimeAdapter,
    RuntimeCapabilities, RuntimeDescriptor, RuntimeKind, RuntimeObserver,
    ValidatedExecutionRequest,
};
use world_domain::{Grams, ParameterProperty, Seconds, ValidatedWorldDefinition};

use crate::{
    MujocoAdapterError, MujocoBinding, MujocoProcessCommand,
    compiler::compile_request,
    contract::{AdapterContractSchemaVersion, AdapterExecutionResponse},
};

const MUJOCO_ENGINE_VERSION: &str = "3.7.0";

/// Capability-checked `MuJoCo` runtime for a single verified world package.
#[derive(Clone, Debug)]
pub struct MujocoAdapter {
    package_root: PathBuf,
    binding: MujocoBinding,
    process: MujocoProcessCommand,
    capabilities: RuntimeCapabilities,
}

impl MujocoAdapter {
    /// Creates an adapter with an explicit package binding and process command.
    pub fn new(
        package_root: impl Into<PathBuf>,
        binding: MujocoBinding,
        process: MujocoProcessCommand,
    ) -> Self {
        Self {
            package_root: package_root.into(),
            binding,
            process,
            capabilities: runtime_capabilities(),
        }
    }

    /// Returns the canonical world-package directory bound to this adapter.
    pub fn package_root(&self) -> &Path {
        &self.package_root
    }

    fn validate_response(
        response: &AdapterExecutionResponse,
        request: &ValidatedExecutionRequest,
    ) -> Result<(), MujocoAdapterError> {
        if response.schema_version != AdapterContractSchemaVersion::V1 {
            return Err(MujocoAdapterError::ContractViolation(
                "response schema version does not match the request contract".to_owned(),
            ));
        }
        if response.engine_version != MUJOCO_ENGINE_VERSION {
            return Err(MujocoAdapterError::ContractViolation(format!(
                "worker reported MuJoCo {}; expected {MUJOCO_ENGINE_VERSION}",
                response.engine_version
            )));
        }

        let limits = request.request().limits;
        if response.summary.executed_steps > limits.maximum_steps {
            return Err(MujocoAdapterError::ContractViolation(format!(
                "worker executed {} steps; limit is {}",
                response.summary.executed_steps, limits.maximum_steps
            )));
        }
        validate_non_negative_duration("logical_duration", response.summary.logical_duration)?;
        validate_non_negative_duration(
            "wall_clock_duration",
            response.summary.wall_clock_duration,
        )?;
        if response.summary.wall_clock_duration.get() > limits.wall_clock_timeout.get() {
            return Err(MujocoAdapterError::ContractViolation(format!(
                "worker reported {:.6}s wall time; limit is {:.6}s",
                response.summary.wall_clock_duration.get(),
                limits.wall_clock_timeout.get()
            )));
        }
        for event in &response.events {
            if let runtime_api::RuntimeEvent::MassMeasurement { value, .. } = event {
                validate_mass(*value)?;
            }
        }
        Ok(())
    }
}

impl RuntimeAdapter for MujocoAdapter {
    type Error = MujocoAdapterError;

    fn capabilities(&self) -> &RuntimeCapabilities {
        &self.capabilities
    }

    fn execute<O: RuntimeObserver>(
        &mut self,
        world: &ValidatedWorldDefinition,
        request: &ValidatedExecutionRequest,
        observer: &mut O,
    ) -> Result<runtime_api::ExecutionSummary, Self::Error> {
        self.capabilities.supports_world(world)?;
        let compiled = compile_request(world, request, &self.binding, &self.package_root)?;
        if !compiled.scene_path.is_file() {
            return Err(MujocoAdapterError::MissingSceneAsset(
                self.binding.scene_asset.to_string(),
            ));
        }
        let response = self.process.execute(&compiled)?;
        Self::validate_response(&response, request)?;
        for event in response.events {
            observer
                .observe(event)
                .map_err(|source| MujocoAdapterError::Observer(Box::new(source)))?;
        }
        Ok(response.summary)
    }
}

fn runtime_capabilities() -> RuntimeCapabilities {
    RuntimeCapabilities {
        descriptor: RuntimeDescriptor {
            adapter_name: "robokit-mujoco".to_owned(),
            adapter_version: env!("CARGO_PKG_VERSION").to_owned(),
            engine_name: "MuJoCo".to_owned(),
            engine_version: MUJOCO_ENGINE_VERSION.to_owned(),
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
            EventCapability::UnstableMeasurement,
        ],
    }
}

fn validate_non_negative_duration(
    field: &str,
    duration: Seconds,
) -> Result<(), MujocoAdapterError> {
    if duration.is_non_negative_finite() {
        Ok(())
    } else {
        Err(MujocoAdapterError::ContractViolation(format!(
            "worker returned invalid {field}"
        )))
    }
}

fn validate_mass(value: Grams) -> Result<(), MujocoAdapterError> {
    if value.is_non_negative_finite() {
        Ok(())
    } else {
        Err(MujocoAdapterError::ContractViolation(
            "worker returned an invalid mass measurement".to_owned(),
        ))
    }
}
