use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use world_domain::{EventKind, Grams, MeasurementId, Seconds};

/// One mass reading produced by a protocol.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct MassMeasurement {
    /// Measurement identity declared by the protocol.
    pub measurement_id: MeasurementId,
    /// Observed mass.
    pub value: Grams,
}

/// Typed scientific outcome produced by either runtime kind.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct RunOutcome {
    /// Gravimetric observations.
    pub mass_measurements: Vec<MassMeasurement>,
    /// Scientific, operational, and safety events.
    pub events: Vec<EventKind>,
    /// Duration advanced on the task timeline.
    pub duration: Seconds,
}
