use tempfile::tempdir;

use super::super::packing::pack::pack_bundle;
use super::super::types::PackBundleRequest;
use super::materialize::{create_staging_installation, materialize_analysis_to_installation};
use super::prepare::prepare_external_package_artifacts;
use super::types::{CreateExternalPackageBundleRequest, PreparedExternalPackageBundle};
use crate::core::error::AppResult;
use crate::core::install::HostPlatform;

pub fn create_external_package_bundle(
    request: CreateExternalPackageBundleRequest,
) -> AppResult<PreparedExternalPackageBundle> {
    let (analysis, manifest) = prepare_external_package_artifacts(&request)?;

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
