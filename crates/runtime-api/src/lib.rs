#![doc = "Bounded engine-neutral execution capabilities for simulators and physical robots."]

mod capability;
mod execution;

pub use capability::{
    CapabilityMismatch, MaterialCapability, ProtocolActionCapability, RuntimeCapabilities,
    RuntimeKind,
};
pub use execution::{
    ExecutionLimits, ExecutionRequest, ExecutionSummary, RuntimeAdapter, RuntimeEvent,
    RuntimeObserver, RuntimeRequestError, ValidatedExecutionRequest,
};
