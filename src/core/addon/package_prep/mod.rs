mod archive;
mod inspect;
mod package_id;

use std::path::Path;

use tempfile::{TempDir, tempdir};

use super::provider::{
    AddonProvider, AddonProviderContext, MaterializeSourceInputRequest, MaterializeSourceRefRequest,
};
use super::{AddonSourceRef, PreparedAddonDirectory, PreparedAddonPackage, TrackedAddon};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::task::CancellationToken;

use self::archive::extract_archive_addons;
pub(crate) use self::inspect::find_primary_toc;
use self::package_id::derive_package_id;

pub(crate) fn prepare_package_from_source_input_with_provider<P>(
    provider: &P,
    source: &str,
    target_flavor: Option<WowFlavor>,
    target_platform: HostPlatform,
    cancellation: &dyn CancellationToken,
) -> AppResult<PreparedAddonPackage>
where
    P: AddonProvider + ?Sized,
{
    let stage_dir = tempdir()?;
    let materialized = provider.materialize_source_input(MaterializeSourceInputRequest {
        source,
        stage_root: stage_dir.path(),
        context: AddonProviderContext {
            target_flavor,
            cancellation: Some(cancellation),
        },
    })?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        target_platform,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_source_ref_with_provider<P>(
    provider: &P,
    source: &AddonSourceRef,
    target_flavor: Option<WowFlavor>,
    target_platform: HostPlatform,
    cancellation: &dyn CancellationToken,
) -> AppResult<PreparedAddonPackage>
where
    P: AddonProvider + ?Sized,
{
    let stage_dir = tempdir()?;
    let materialized = provider.materialize_source_ref(MaterializeSourceRefRequest {
        source,
        stage_root: stage_dir.path(),
        context: AddonProviderContext {
            target_flavor,
            cancellation: Some(cancellation),
        },
    })?;
    prepare_package_from_archive(
        materialized.source_ref,
        &materialized.archive_path,
        target_platform,
        stage_dir,
    )
}

pub(crate) fn prepare_package_from_archive_with_source(
    source: AddonSourceRef,
    archive_path: &Path,
    target_platform: HostPlatform,
) -> AppResult<PreparedAddonPackage> {
    let stage_dir = tempdir()?;
    prepare_package_from_archive(source, archive_path, target_platform, stage_dir)
}

fn prepare_package_from_archive(
    source: AddonSourceRef,
    archive_path: &Path,
    target_platform: HostPlatform,
    stage_dir: TempDir,
) -> AppResult<PreparedAddonPackage> {
    let addons = extract_archive_addons(archive_path, stage_dir.path(), target_platform)?;
    if addons.is_empty() {
        return Err(AppError::Validation(
            "archive does not contain any detectable addon directories".to_string(),
        ));
    }

    let addon_names = addons
        .iter()
        .map(|addon| addon.addon.directory_name.as_str())
        .collect::<Vec<_>>();
    let package_id = derive_package_id(&source, &addon_names);

    Ok(PreparedAddonPackage {
        source,
        package_id,
        addons,
        metadata: None,
        _stage_dir: stage_dir,
    })
}
