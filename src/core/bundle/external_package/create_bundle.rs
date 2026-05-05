use tempfile::tempdir;

use super::super::packing::pack::pack_bundle;
use super::super::types::archive::PackBundleRequest;
use super::materialize::{create_staging_installation, materialize_analysis_to_installation};
use super::prepare::prepare_external_package_artifacts;
use super::types::{
    CreateExternalPackageBundleRequest, ExternalPackageAnalysis,
    ExternalPackagePublicSharingSeverity, ExternalPackageSharingMode,
    PreparedExternalPackageBundle,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::HostPlatform;

pub fn create_external_package_bundle(
    request: CreateExternalPackageBundleRequest,
) -> AppResult<PreparedExternalPackageBundle> {
    let (analysis, manifest) = prepare_external_package_artifacts(&request)?;
    validate_public_sharing_policy(&request, &analysis)?;

    let stage_dir = tempdir()?;
    let staged_installation = create_staging_installation(
        stage_dir.path(),
        request.source_flavor,
        request
            .source_platform
            .unwrap_or_else(HostPlatform::current),
    )?;
    materialize_analysis_to_installation(&analysis, &staged_installation)?;

    let output_path = request
        .output_path
        .clone()
        .or_else(|| Some(stage_dir.path().join("external-package.bundle.zip")));
    let bundle = pack_bundle(PackBundleRequest {
        installation: staged_installation,
        addon_state_storage_kind: crate::core::addon::AddonStateStorageKind::default(),
        manifest: manifest.clone(),
        output_path,
        manifest_base_dir: None,
    })?;

    Ok(PreparedExternalPackageBundle {
        analysis,
        manifest,
        bundle,
        _stage_dir: stage_dir,
    })
}

fn validate_public_sharing_policy(
    request: &CreateExternalPackageBundleRequest,
    analysis: &ExternalPackageAnalysis,
) -> AppResult<()> {
    if request.sharing_mode != ExternalPackageSharingMode::Public
        || request.allow_public_sharing_risks
        || analysis.summary.public_sharing.public_ready
    {
        return Ok(());
    }

    let reasons = analysis
        .summary
        .public_sharing
        .reasons
        .iter()
        .filter(|reason| reason.severity == ExternalPackagePublicSharingSeverity::ReviewRequired)
        .map(|reason| format!("{}={}", reason.code.as_str(), reason.count))
        .collect::<Vec<_>>()
        .join(", ");

    Err(AppError::Validation(format!(
        "public sharing export requires review before bundle creation: {reasons}; use private sharing mode or explicitly allow public sharing risks after review"
    )))
}
