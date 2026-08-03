#![doc = "Bounded engine-neutral execution capabilities for simulators and physical robots."]

mod capability;
mod execution;

pub use capability::{
    CapabilityMismatch, CapabilityRequirement, EventCapability, MaterialCapability,
    ProtocolActionCapability, RuntimeCapabilities, RuntimeDescriptor, RuntimeKind,
};
pub use execution::{
    ExecutionLimits, ExecutionRequest, ExecutionSummary, RuntimeAdapter, RuntimeEvent,
    RuntimeObserver, RuntimeRequestError, ValidatedExecutionRequest,
};
