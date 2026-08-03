#![doc = "Deterministic evaluation shared by simulated and physical runs."]

mod outcome;
mod report;

pub use outcome::{MassMeasurement, RunOutcome};
pub use report::{
    EvaluationError, EvaluationReport, VerifierKind, VerifierResult, evaluate_outcome,
};
