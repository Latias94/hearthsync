use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;
use crate::core::manifest::BundleManifest;

pub(in crate::core::bundle) fn validate_target_compatibility(
    manifest: &BundleManifest,
    installation: &DetectedFlavorInstallation,
) -> AppResult<()> {
    if !manifest.source.supported_targets.is_empty()
        && !manifest
            .source
            .supported_targets
            .contains(&installation.flavor)
    {
        return Err(AppError::Validation(format!(
            "bundle does not support target flavor `{}`",
            installation.flavor.as_str()
        )));
    }

    if let Some(source_platform) = manifest.source.platform
        && source_platform != installation.platform
        && !manifest.mapping.allow_cross_platform
    {
        return Err(AppError::Validation(
            "bundle was exported on another platform, but allow_cross_platform is false"
                .to_string(),
        ));
    }

    Ok(())
}
