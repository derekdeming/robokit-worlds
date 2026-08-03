#![doc = "Validated physical-data ingestion and scientific-world calibration."]

mod dataset;
mod error;
mod fit;
mod report;

pub use dataset::{
    CalibrationDatasetSchemaVersion, CalibrationWorldRevision, GranularCalibrationDataset,
    GranularDosingObservation, ValidatedGranularCalibrationDataset,
};
pub use error::{CalibrationError, DatasetValidationError, DatasetValidationIssue};
pub use fit::{CalibratedGranularWorld, GranularFit, ParameterEstimate, calibrate_granular_world};
pub use report::{
    CalibratedWorldRevision, CalibrationMethod, CalibrationReport, CalibrationReportSchemaVersion,
};
