use std::fs;
use std::path::Path;

use super::*;
use crate::core::error::{AppError, AppResult};

pub fn inspect_addon_index(path: &Path) -> AppResult<AddonIndexInspection> {
    let index = load_addon_index(path)?;
    let package_count = index.packages.len();

    Ok(AddonIndexInspection {
        index_path: path.to_path_buf(),
        index,
        package_count,
    })
}

pub(super) fn load_addon_index(path: &Path) -> AppResult<AddonIndex> {
    let content = fs::read_to_string(path)?;
    let index = toml::from_str::<AddonIndex>(&content)?;
    validate_addon_index(&index)?;
    Ok(index)
}

fn validate_addon_index(index: &AddonIndex) -> AppResult<()> {
    if index.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon index schema version: {}",
            index.schema_version
        )));
    }
    if index.name.trim().is_empty() {
        return Err(AppError::Validation(
            "addon index name must not be empty".to_string(),
        ));
    }
    if index.packages.is_empty() {
        return Err(AppError::Validation(
            "addon index must contain at least one package".to_string(),
        ));
    }

    let mut ids = Vec::new();
    for package in &index.packages {
        validate_index_package(package)?;
        if ids.iter().any(|id| id == &package.id) {
            return Err(AppError::Validation(format!(
                "duplicate addon index package id: {}",
                package.id
            )));
        }
        ids.push(package.id.clone());
    }

    Ok(())
}

fn validate_index_package(package: &AddonIndexPackage) -> AppResult<()> {
    for (field, value) in [
        ("package id", &package.id),
        ("package name", &package.name),
        ("package version", &package.version),
    ] {
        if value.trim().is_empty() {
            return Err(AppError::Validation(format!("{field} must not be empty")));
        }
    }

    for flavor in &package.supported_flavors {
        if flavor.trim().is_empty() {
            return Err(AppError::Validation(format!(
                "supported flavor must not be empty for package `{}`",
                package.id
            )));
        }
    }

    Ok(())
}

pub(super) fn find_index_package<'a>(
    index: &'a AddonIndex,
    name: &str,
) -> AppResult<&'a AddonIndexPackage> {
    index
        .packages
        .iter()
        .find(|package| {
            package.id.eq_ignore_ascii_case(name) || package.name.eq_ignore_ascii_case(name)
        })
        .ok_or_else(|| AppError::NotFound(format!("addon index package `{name}` not found")))
}

pub(super) fn ensure_package_supports_flavor(
    package: &AddonIndexPackage,
    flavor: &str,
) -> AppResult<()> {
    if package.supported_flavors.is_empty()
        || package
            .supported_flavors
            .iter()
            .any(|item| item.eq_ignore_ascii_case(flavor))
    {
        return Ok(());
    }

    Err(AppError::Validation(format!(
        "package `{}` does not support flavor `{}`. Supported flavors: {}",
        package.id,
        flavor,
        package.supported_flavors.join(", ")
    )))
}
