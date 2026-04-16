use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::core::error::{AppError, AppResult};

use super::layout::{
    build_installation, candidate_product_roots, detect_flavors, has_wow_structure, normalize_path,
};
use super::model::{
    DetectedFlavorInstallation, HealthStatus, InstallationHealth, ProductInstallInspection,
    WowFlavor,
};

pub fn scan_installations() -> AppResult<Vec<DetectedFlavorInstallation>> {
    let mut results = Vec::new();
    let mut seen = BTreeSet::new();

    for product_root in candidate_product_roots() {
        if !product_root.exists() {
            continue;
        }

        for flavor in detect_flavors(&product_root) {
            let installation = build_installation(
                &product_root,
                &product_root.join(flavor.folder_name()),
                flavor,
            );
            if seen.insert(installation.flavor_root.clone()) {
                results.push(installation);
            }
        }
    }

    results.sort_by(|left, right| {
        left.product_root
            .cmp(&right.product_root)
            .then(left.flavor.cmp(&right.flavor))
    });

    Ok(results)
}

pub fn inspect_installation(
    path: &Path,
    flavor: Option<WowFlavor>,
) -> AppResult<ProductInstallInspection> {
    let requested_path = normalize_path(path)?;
    let (product_root, available_flavors, preselected) =
        classify_installation_path(&requested_path, flavor)?;

    let selected_flavor = match (flavor, preselected, available_flavors.as_slice()) {
        (Some(explicit), _, available) if available.contains(&explicit) => explicit,
        (Some(explicit), Some(actual), _) if explicit == actual => explicit,
        (Some(explicit), _, _) => {
            return Err(AppError::NotFound(format!(
                "Flavor `{}` is not available under `{}`",
                explicit.as_str(),
                requested_path.display()
            )));
        }
        (None, Some(actual), _) => actual,
        (None, _, [single]) => *single,
        (None, _, many) => {
            return Err(AppError::AmbiguousFlavor {
                path: requested_path,
                detected: many.iter().map(|item| item.as_str().to_string()).collect(),
            });
        }
    };

    let installation =
        resolve_detected_installation(&product_root, &requested_path, selected_flavor)?;
    let health = evaluate_installation(&installation);

    Ok(ProductInstallInspection {
        requested_path: installation.flavor_root.clone(),
        product_root,
        available_flavors,
        installation,
        health,
    })
}

pub fn resolve_installation(
    path: &Path,
    flavor: Option<WowFlavor>,
) -> AppResult<DetectedFlavorInstallation> {
    inspect_installation(path, flavor).map(|item| item.installation)
}

fn classify_installation_path(
    path: &Path,
    flavor: Option<WowFlavor>,
) -> AppResult<(PathBuf, Vec<WowFlavor>, Option<WowFlavor>)> {
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    if let Some(detected_flavor) = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(WowFlavor::from_folder_name)
    {
        let product_root = path.parent().map(Path::to_path_buf).ok_or_else(|| {
            AppError::Validation(format!(
                "Unable to determine product root from `{}`",
                path.display()
            ))
        })?;

        let available_flavors = detect_flavors(&product_root);
        let flavors = if available_flavors.is_empty() {
            vec![detected_flavor]
        } else {
            available_flavors
        };

        return Ok((product_root, flavors, Some(detected_flavor)));
    }

    let detected_flavors = detect_flavors(path);
    if !detected_flavors.is_empty() {
        return Ok((path.to_path_buf(), detected_flavors, None));
    }

    if has_wow_structure(path) {
        let explicit_flavor = flavor.ok_or_else(|| {
            AppError::Validation(format!(
                "Path `{}` looks like a flavor root, but its folder name is unknown. Please pass `--flavor`.",
                path.display()
            ))
        })?;

        let product_root = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf());
        return Ok((product_root, vec![explicit_flavor], Some(explicit_flavor)));
    }

    Err(AppError::NotFound(format!(
        "Path is not a recognizable WoW installation: {}",
        path.display()
    )))
}

fn resolve_detected_installation(
    product_root: &Path,
    requested_path: &Path,
    flavor: WowFlavor,
) -> AppResult<DetectedFlavorInstallation> {
    let flavor_root = if requested_path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(WowFlavor::from_folder_name)
        == Some(flavor)
    {
        requested_path.to_path_buf()
    } else if has_wow_structure(requested_path)
        && !requested_path.join(flavor.folder_name()).exists()
    {
        requested_path.to_path_buf()
    } else {
        product_root.join(flavor.folder_name())
    };

    if !flavor_root.exists() {
        return Err(AppError::NotFound(format!(
            "Flavor root does not exist: {}",
            flavor_root.display()
        )));
    }

    Ok(build_installation(product_root, &flavor_root, flavor))
}

fn evaluate_installation(installation: &DetectedFlavorInstallation) -> InstallationHealth {
    let mut missing_paths = Vec::new();
    let mut warnings = Vec::new();

    for required in [
        installation.flavor_root.as_path(),
        installation.interface_dir.as_path(),
        installation.addon_dir.as_path(),
        installation.wtf_dir.as_path(),
    ] {
        if !required.exists() {
            missing_paths.push(required.to_path_buf());
        }
    }

    if !installation.fonts_dir.exists() {
        warnings.push(format!(
            "Optional fonts directory is missing: {}",
            installation.fonts_dir.display()
        ));
    }

    let config_wtf = installation.wtf_dir.join("Config.wtf");
    if !config_wtf.exists() {
        warnings.push(format!(
            "Config.wtf was not found under `{}`",
            installation.wtf_dir.display()
        ));
    }

    let status = if !missing_paths.is_empty() {
        HealthStatus::Broken
    } else if warnings.is_empty() {
        HealthStatus::Healthy
    } else {
        HealthStatus::Warning
    };

    InstallationHealth {
        status,
        missing_paths,
        warnings,
    }
}
