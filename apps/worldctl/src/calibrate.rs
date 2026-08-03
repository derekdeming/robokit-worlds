use std::{collections::HashSet, path::PathBuf};

use calibration::{
    CalibratedWorldRevision, CalibrationMethod, CalibrationReport, CalibrationReportSchemaVersion,
    GranularCalibrationDataset, ValidatedGranularCalibrationDataset, calibrate_granular_world,
};
use world_domain::{PackagePath, Sha256Digest, ValidatedWorldDefinition};

use crate::{
    error::CliError,
    files::{
        StagedDirectory, copy_file, parse_json, pretty_json, read_bounded_bytes, verify_digest,
        verify_digest_file, verify_world_assets, write_file,
    },
};

const WORLD_FILE_NAME: &str = "world.toml";
const REPORT_FILE_NAME: &str = "calibration-report.json";
const DATASET_PACKAGE_PATH: &str = "calibration/source-dataset.json";
const SOURCE_WORLD_PACKAGE_PATH: &str = "calibration/source-world.toml";
const MAXIMUM_DATASET_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_REPORT_BYTES: u64 = 2 * 1024 * 1024;
const MAXIMUM_WORLD_BYTES: u64 = 4 * 1024 * 1024;

/// Explicit inputs for one immutable granular calibration publication.
pub(crate) struct CalibrationOptions {
    pub(crate) world_path: PathBuf,
    pub(crate) dataset_path: PathBuf,
    pub(crate) new_version: String,
    pub(crate) output_directory: PathBuf,
}

pub(crate) fn calibrate_granular(options: CalibrationOptions) -> Result<(), CliError> {
    let output = StagedDirectory::create(options.output_directory)?;
    let source_world_bytes = read_bounded_bytes(&options.world_path, MAXIMUM_WORLD_BYTES)?;
    let source_world_text =
        std::str::from_utf8(&source_world_bytes).map_err(|error| CliError::InvalidTextInput {
            path: options.world_path.clone(),
            reason: error.to_string(),
        })?;
    let source_world = crate::files::parse_world(&options.world_path, source_world_text)?;
    verify_world_assets(&options.world_path, &source_world)?;
    let source_world_digest = evidence::sha256_digest(&source_world_bytes);
    let dataset_bytes = read_bounded_bytes(&options.dataset_path, MAXIMUM_DATASET_BYTES)?;
    let dataset: GranularCalibrationDataset = parse_json(&options.dataset_path, &dataset_bytes)?;
    let dataset = validate_dataset(dataset, &source_world, &source_world_digest)?;
    let calibrated = calibrate_granular_world(&source_world, &dataset, &options.new_version)?;
    let world_bytes = calibrated_world_toml(calibrated.world().definition())?;
    let calibrated_world_digest = evidence::sha256_digest(&world_bytes);
    let dataset_digest = evidence::sha256_digest(&dataset_bytes);

    copy_world_assets(
        &options.world_path,
        calibrated.world().definition(),
        output.path(),
    )?;
    write_file(&output.path().join(WORLD_FILE_NAME), &world_bytes)?;
    let packaged_dataset_path = output.path().join(DATASET_PACKAGE_PATH);
    if let Some(parent) = packaged_dataset_path.parent() {
        crate::files::create_directory(parent)?;
    }
    write_file(&packaged_dataset_path, &dataset_bytes)?;
    write_file(
        &output.path().join(SOURCE_WORLD_PACKAGE_PATH),
        &source_world_bytes,
    )?;

    let report = CalibrationReport {
        schema_version: CalibrationReportSchemaVersion::V1,
        method: CalibrationMethod::GranularLinearFlowV1,
        source_world: dataset.dataset().world.clone(),
        source_world_path: PackagePath::new(SOURCE_WORLD_PACKAGE_PATH),
        calibrated_world: CalibratedWorldRevision {
            id: calibrated.world().definition().id,
            version: calibrated.world().definition().version.clone(),
            definition_sha256: calibrated_world_digest,
        },
        source_dataset_path: PackagePath::new(DATASET_PACKAGE_PATH),
        source_dataset_sha256: dataset_digest,
        material_batch_id: dataset.dataset().material_batch_id.clone(),
        observation_count: observation_count(&dataset)?,
        fit: calibrated.fit().clone(),
    };
    write_file(
        &output.path().join(REPORT_FILE_NAME),
        &pretty_json(&report)?,
    )?;
    output.publish()
}

pub(crate) fn validate_calibration_package(package_root: &std::path::Path) -> Result<(), CliError> {
    let report_path = package_root.join(REPORT_FILE_NAME);
    let report_bytes = read_bounded_bytes(&report_path, MAXIMUM_REPORT_BYTES)?;
    let report: CalibrationReport = parse_json(&report_path, &report_bytes)?;
    validate_report_paths(&report)?;

    let source_world_path = package_root.join(report.source_world_path.as_str());
    let source_world_bytes = read_bounded_bytes(&source_world_path, MAXIMUM_WORLD_BYTES)?;
    verify_digest(
        &source_world_path,
        &source_world_bytes,
        &report.source_world.definition_sha256,
    )?;
    let source_world_text = std::str::from_utf8(&source_world_bytes).map_err(|error| {
        CliError::InvalidCalibrationPackage(format!("source world is not UTF-8: {error}"))
    })?;
    let source_world = crate::files::parse_world(&source_world_path, source_world_text)?;
    validate_world_identity(
        source_world.definition(),
        report.source_world.id,
        &report.source_world.version,
        "source",
    )?;

    let dataset_path = package_root.join(report.source_dataset_path.as_str());
    let dataset_bytes = read_bounded_bytes(&dataset_path, MAXIMUM_DATASET_BYTES)?;
    verify_digest(&dataset_path, &dataset_bytes, &report.source_dataset_sha256)?;
    let dataset: GranularCalibrationDataset = parse_json(&dataset_path, &dataset_bytes)?;
    let dataset = validate_dataset(
        dataset,
        &source_world,
        &report.source_world.definition_sha256,
    )?;
    if report.observation_count != observation_count(&dataset)?
        || report.material_batch_id != dataset.dataset().material_batch_id
    {
        return Err(CliError::InvalidCalibrationPackage(
            "report dataset summary does not match the packaged dataset".to_owned(),
        ));
    }

    let calibrated_world_path = package_root.join(WORLD_FILE_NAME);
    let calibrated_world_bytes = read_bounded_bytes(&calibrated_world_path, MAXIMUM_WORLD_BYTES)?;
    verify_digest(
        &calibrated_world_path,
        &calibrated_world_bytes,
        &report.calibrated_world.definition_sha256,
    )?;
    let calibrated_world_text = std::str::from_utf8(&calibrated_world_bytes).map_err(|error| {
        CliError::InvalidCalibrationPackage(format!("calibrated world is not UTF-8: {error}"))
    })?;
    let calibrated_world =
        crate::files::parse_world(&calibrated_world_path, calibrated_world_text)?;
    verify_world_assets(&calibrated_world_path, &calibrated_world)?;
    validate_world_identity(
        calibrated_world.definition(),
        report.calibrated_world.id,
        &report.calibrated_world.version,
        "calibrated",
    )?;

    let reproduced = calibrate_granular_world(
        &source_world,
        &dataset,
        report.calibrated_world.version.as_str(),
    )?;
    if !reproduced.fit().numerically_matches(&report.fit) {
        return Err(CliError::InvalidCalibrationPackage(
            "packaged fit is not reproducible from its source data".to_owned(),
        ));
    }
    if !reproduced.matches_definition(calibrated_world.definition()) {
        return Err(CliError::InvalidCalibrationPackage(
            "calibrated world is not reproducible from its source data".to_owned(),
        ));
    }
    Ok(())
}

fn validate_dataset(
    dataset: GranularCalibrationDataset,
    source_world: &ValidatedWorldDefinition,
    source_world_digest: &Sha256Digest,
) -> Result<ValidatedGranularCalibrationDataset, CliError> {
    dataset
        .validate_for_world(source_world, source_world_digest)
        .map_err(|error| {
            CliError::InvalidCalibrationDataset(
                error
                    .issues()
                    .iter()
                    .map(|issue| format!("{}: {}", issue.path, issue.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })
}

fn observation_count(dataset: &ValidatedGranularCalibrationDataset) -> Result<u64, CliError> {
    u64::try_from(dataset.dataset().observations.len()).map_err(|_| {
        CliError::InvalidCalibrationPackage(
            "observation count cannot be represented by the package contract".to_owned(),
        )
    })
}

fn validate_report_paths(report: &CalibrationReport) -> Result<(), CliError> {
    if report.source_world_path.is_well_formed() && report.source_dataset_path.is_well_formed() {
        Ok(())
    } else {
        Err(CliError::InvalidCalibrationPackage(
            "report source paths must remain inside the package".to_owned(),
        ))
    }
}

fn validate_world_identity(
    definition: &world_domain::WorldDefinition,
    expected_id: world_domain::WorldId,
    expected_version: &world_domain::WorldVersion,
    label: &str,
) -> Result<(), CliError> {
    if definition.id == expected_id && definition.version == *expected_version {
        Ok(())
    } else {
        Err(CliError::InvalidCalibrationPackage(format!(
            "{label} world identity does not match its report revision"
        )))
    }
}

fn calibrated_world_toml(definition: &world_domain::WorldDefinition) -> Result<Vec<u8>, CliError> {
    let mut bytes = toml::to_string_pretty(definition)?.into_bytes();
    if !bytes.ends_with(b"\n") {
        bytes.push(b'\n');
    }
    Ok(bytes)
}

fn copy_world_assets(
    source_world_path: &std::path::Path,
    definition: &world_domain::WorldDefinition,
    destination_root: &std::path::Path,
) -> Result<(), CliError> {
    let source_root = source_world_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut copied_paths = HashSet::new();
    for asset in definition
        .entities
        .iter()
        .filter_map(|entity| entity.asset.as_ref())
    {
        if !copied_paths.insert(asset.logical_path.as_str()) {
            continue;
        }
        let source_path = source_root.join(asset.logical_path.as_str());
        let destination_path = destination_root.join(asset.logical_path.as_str());
        if let Some(parent) = destination_path.parent() {
            crate::files::create_directory(parent)?;
        }
        copy_file(&source_path, &destination_path)?;
        verify_digest_file(&destination_path, &asset.sha256)?;
    }
    Ok(())
}
