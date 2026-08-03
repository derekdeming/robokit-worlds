use std::collections::HashSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use world_domain::{EntityId, PackagePath};

use crate::MujocoAdapterError;

/// Version of a `MuJoCo` world-binding file.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub enum MujocoBindingSchemaVersion {
    /// Initial binding contract.
    #[serde(rename = "1.0.0")]
    V1,
}

/// Engine-specific names kept outside the canonical scientific world.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MujocoBinding {
    /// Binding contract version.
    pub schema_version: MujocoBindingSchemaVersion,
    /// MJCF asset referenced by the canonical world.
    pub scene_asset: PackagePath,
    /// Canonical robot entity controlled by the adapter.
    pub controlled_entity_id: EntityId,
    /// `MuJoCo` mocap body driven by protocol actions.
    pub controlled_body: String,
    /// Geometries whose contact is reported as a collision.
    pub forbidden_contact_geometries: Vec<String>,
}

impl MujocoBinding {
    /// Parses and performs local shape validation for a TOML binding.
    ///
    /// # Errors
    ///
    /// Returns a typed error for invalid TOML, paths, names, or duplicates.
    pub fn from_toml(value: &str) -> Result<Self, MujocoAdapterError> {
        let binding: Self = toml::from_str(value).map_err(MujocoAdapterError::BindingToml)?;
        binding.validate_shape()?;
        Ok(binding)
    }

    fn validate_shape(&self) -> Result<(), MujocoAdapterError> {
        if !self.scene_asset.is_well_formed() {
            return Err(MujocoAdapterError::InvalidBinding(
                "scene_asset must be a package-contained relative path".to_owned(),
            ));
        }
        if !is_valid_mujoco_name(&self.controlled_body) {
            return Err(MujocoAdapterError::InvalidBinding(
                "controlled_body must be a non-empty MuJoCo name".to_owned(),
            ));
        }
        let mut names = HashSet::new();
        for name in &self.forbidden_contact_geometries {
            if !is_valid_mujoco_name(name) {
                return Err(MujocoAdapterError::InvalidBinding(
                    "forbidden contact geometry names must not be empty".to_owned(),
                ));
            }
            if !names.insert(name) {
                return Err(MujocoAdapterError::InvalidBinding(format!(
                    "forbidden contact geometry `{name}` is duplicated"
                )));
            }
        }
        Ok(())
    }
}

fn is_valid_mujoco_name(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 255 && !value.chars().any(char::is_control)
}
