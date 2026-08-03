use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable machine-readable category for invalid world data.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCode {
    /// Identifier contains invalid characters or length.
    InvalidIdentifier,
    /// Identifier is repeated inside a uniqueness scope.
    DuplicateIdentifier,
    /// A referenced entity or measurement does not exist.
    MissingReference,
    /// A reference resolves to an entity with the wrong capability.
    InvalidReferenceType,
    /// Numeric value is non-finite or outside its permitted range.
    InvalidValue,
    /// Version does not follow semantic versioning.
    InvalidVersion,
    /// Required collection or string is empty.
    EmptyValue,
    /// Asset path is absolute or escapes its world package.
    UnsafeAssetPath,
    /// Asset digest is not lowercase SHA-256 hexadecimal.
    InvalidAssetDigest,
}

/// One validation failure with a stable path and category.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
pub struct ValidationIssue {
    /// Dot-separated location in the world definition.
    pub path: String,
    /// Stable failure category.
    pub code: ValidationCode,
    /// Actionable human-readable diagnostic.
    pub message: String,
}

impl ValidationIssue {
    pub(crate) fn new(
        path: impl Into<String>,
        code: ValidationCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            code,
            message: message.into(),
        }
    }
}

/// Aggregated boundary-validation failures for one world definition.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("world definition has {count} validation issue(s)", count = .issues.len())]
pub struct WorldValidationErrors {
    issues: Vec<ValidationIssue>,
}

impl WorldValidationErrors {
    pub(crate) fn new(issues: Vec<ValidationIssue>) -> Self {
        Self { issues }
    }

    /// Returns every detected issue in deterministic traversal order.
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Consumes the error and returns every issue.
    pub fn into_issues(self) -> Vec<ValidationIssue> {
        self.issues
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({:?}): {}",
            self.path, self.code, self.message
        )
    }
}
