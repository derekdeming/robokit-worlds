use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{EntityId, Meters, PackagePath};

/// Content digest for an immutable asset.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Sha256Digest(String);

impl Sha256Digest {
    /// Creates a digest without validating its enclosing world.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the lowercase hexadecimal digest.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether the digest is canonical lowercase SHA-256 hexadecimal.
    pub fn is_well_formed(&self) -> bool {
        self.0.len() == 64
            && self
                .0
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }
}

/// File format of a runtime asset.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetFormat {
    /// `MuJoCo` XML scene or component.
    Mjcf,
    /// Unified Robot Description Format.
    Urdf,
    /// Universal Scene Description.
    Usd,
    /// Polygon mesh asset.
    Mesh,
}

/// Content-addressed runtime asset owned by a world package.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct AssetReference {
    /// Relative logical path within the world package.
    pub logical_path: PackagePath,
    /// On-disk or interchange representation.
    pub format: AssetFormat,
    /// Digest of the exact asset bytes.
    pub sha256: Sha256Digest,
}

/// Three-dimensional vector.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Vector3 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vector3 {
    /// Returns whether every component is finite.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

/// Unit quaternion in x-y-z-w order.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Quaternion {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
    /// Scalar component.
    pub w: f64,
}

impl Quaternion {
    pub(crate) fn is_normalized(self) -> bool {
        let norm_squared = self.x.mul_add(
            self.x,
            self.y
                .mul_add(self.y, self.z.mul_add(self.z, self.w * self.w)),
        );
        norm_squared.is_finite() && (norm_squared - 1.0).abs() <= 1.0e-6
    }
}

/// Pose expressed in the world coordinate frame.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Pose {
    /// Translation from the world origin.
    pub position: Position3,
    /// Orientation in x-y-z-w order.
    pub orientation: Quaternion,
}

/// Position with meter-valued components.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Position3 {
    /// X position.
    pub x: Meters,
    /// Y position.
    pub y: Meters,
    /// Z position.
    pub z: Meters,
}

impl Position3 {
    pub(crate) fn is_finite(self) -> bool {
        self.x.get().is_finite() && self.y.get().is_finite() && self.z.get().is_finite()
    }
}

/// Supported robot embodiment categories.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RobotEmbodiment {
    /// Fixed-base articulated manipulator.
    Manipulator,
}

/// Supported scientific instrument categories.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentKind {
    /// Gravimetric scale or balance.
    Scale,
    /// RGB camera.
    RgbCamera,
    /// RGB-D camera.
    RgbdCamera,
}

/// Role of a vessel in the benchmark protocol.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VesselRole {
    /// Vessel initially containing material.
    Source,
    /// Vessel receiving material.
    Target,
}

/// Broad material behavior class required by a runtime.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialClass {
    /// Discrete, pourable granular material.
    Granular,
}

/// Semantic kind of an entity.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntityKind {
    /// Robot capable of executing protocol actions.
    Robot {
        /// Physical arrangement of the robot.
        embodiment: RobotEmbodiment,
    },
    /// Scientific measurement instrument.
    Instrument {
        /// Instrument capability.
        instrument: InstrumentKind,
    },
    /// Material-holding vessel.
    Vessel {
        /// Protocol role of the vessel.
        role: VesselRole,
    },
    /// Process material.
    Material {
        /// Modeled behavior category.
        class: MaterialClass,
    },
    /// Static work surface.
    WorkSurface,
}

/// One semantic entity in a scientific world.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct WorldEntity {
    /// Stable identifier referenced by protocols and parameters.
    pub id: EntityId,
    /// Human-readable label.
    pub name: String,
    /// Scientific or operational role.
    pub entity: EntityKind,
    /// Initial pose in the world coordinate frame.
    pub initial_pose: Pose,
    /// Optional content-addressed runtime asset.
    pub asset: Option<AssetReference>,
}
