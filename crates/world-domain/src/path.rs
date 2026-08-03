use std::{fmt, path::Component, path::Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Relative, package-contained path used by content-addressed assets and artifacts.
#[derive(Clone, Debug, Deserialize, Hash, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PackagePath(String);

impl PackagePath {
    /// Creates a package path without validating its enclosing contract.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the serialized relative path.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether this path is non-empty, relative, and package-contained.
    pub fn is_well_formed(&self) -> bool {
        let path = Path::new(&self.0);
        !self.0.is_empty()
            && !path.is_absolute()
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
    }
}

impl fmt::Display for PackagePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_path_rejects_absolute_and_parent_traversal() {
        assert!(PackagePath::new("assets/world.xml").is_well_formed());
        assert!(!PackagePath::new("../world.xml").is_well_formed());
        assert!(!PackagePath::new("/tmp/world.xml").is_well_formed());
    }
}
