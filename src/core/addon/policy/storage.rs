use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::core::addon::{TrackedAddonPackage, load_registry, select_tracked_packages};
use crate::core::atomic_write::write_bytes_atomically;
use crate::core::error::{AppError, AppResult};
use crate::core::install::DetectedFlavorInstallation;

use super::{
    AddonPolicyInspection, AddonPolicyMutationResult, AddonPolicyPackageEntry,
    AddonPolicyPackageView, AddonPolicyPin, AddonPolicyState, RemoveAddonPolicyRequest,
    SetAddonPolicyRequest, normalize_package_key,
};

pub fn inspect_addon_policy(
    installation: &DetectedFlavorInstallation,
    state_paths: &crate::core::addon::AddonStatePaths,
) -> AppResult<AddonPolicyInspection> {
    let path = policy_path(state_paths);
    let state = read_addon_policy_state(&path)?;
    let registry = load_registry(installation, state_paths)?;
    let package_count = state.packages.len();
    let mut packages = state
        .packages
        .iter()
        .map(|entry| {
            AddonPolicyPackageView::from_entry(
                entry,
                registry
                    .packages
                    .iter()
                    .find(|package| package.package_id.eq_ignore_ascii_case(&entry.package_id)),
            )
        })
        .collect::<Vec<_>>();
    packages.sort_by(|left, right| left.package_id.cmp(&right.package_id));

    Ok(AddonPolicyInspection {
        policy_path: path,
        package_count,
        packages,
    })
}

pub fn set_addon_policy(request: SetAddonPolicyRequest) -> AppResult<AddonPolicyMutationResult> {
    validate_set_request(&request)?;

    let path = policy_path(&request.state_paths);
    let mut state = read_addon_policy_state(&path)?;
    let registry = load_registry(&request.installation, &request.state_paths)?;
    let (package_id, tracked_package) =
        resolve_policy_target(&registry.packages, &state, request.package.trim())?;
    let pin = build_requested_pin(&request)?;
    let entry_index = state
        .packages
        .iter()
        .position(|entry| entry.package_id.eq_ignore_ascii_case(&package_id));
    let mut entry = entry_index
        .map(|index| state.packages[index].clone())
        .unwrap_or_else(|| AddonPolicyPackageEntry::new(package_id.clone()));

    if let Some(ignored) = request.ignored {
        entry.ignored = Some(ignored);
    }
    if let Some(pin) = pin {
        entry.pin = Some(pin);
    }
    if let Some(release_channel) = request.release_channel {
        entry.release_channel = Some(release_channel);
    }
    if let Some(allow_prerelease) = request.allow_prerelease {
        entry.allow_prerelease = Some(allow_prerelease);
    }
    if let Some(install_dependencies) = request.install_dependencies {
        entry.install_dependencies = Some(install_dependencies);
    }

    if let Some(index) = entry_index {
        state.packages[index] = entry.clone();
    } else {
        state.packages.push(entry.clone());
    }
    persist_addon_policy_state(&path, &mut state)?;

    Ok(AddonPolicyMutationResult {
        policy_path: path,
        package_count: state.packages.len(),
        package_id,
        entry_removed: false,
        package: Some(AddonPolicyPackageView::from_entry(
            &entry,
            tracked_package.as_ref(),
        )),
    })
}

pub fn remove_addon_policy(
    request: RemoveAddonPolicyRequest,
) -> AppResult<AddonPolicyMutationResult> {
    let package = request.package.trim();
    if package.is_empty() {
        return Err(AppError::Validation(
            "addon policy package selector cannot be empty".to_string(),
        ));
    }

    let path = policy_path(&request.state_paths);
    let mut state = read_addon_policy_state(&path)?;
    let registry = load_registry(&request.installation, &request.state_paths)?;
    let (package_id, _) = resolve_policy_target(&registry.packages, &state, package)?;
    let original_len = state.packages.len();
    state
        .packages
        .retain(|entry| !entry.package_id.eq_ignore_ascii_case(&package_id));

    if state.packages.len() == original_len {
        return Err(AppError::NotFound(format!(
            "no addon policy entry matched `{package}`"
        )));
    }

    persist_addon_policy_state(&path, &mut state)?;
    Ok(AddonPolicyMutationResult {
        policy_path: path,
        package_count: state.packages.len(),
        package_id,
        entry_removed: true,
        package: None,
    })
}

pub fn policy_path(state_paths: &crate::core::addon::AddonStatePaths) -> PathBuf {
    state_paths.policy_path.clone()
}

pub(crate) fn load_addon_policy_state(
    installation: &DetectedFlavorInstallation,
    state_paths: &crate::core::addon::AddonStatePaths,
) -> AppResult<AddonPolicyState> {
    let _ = installation;
    read_addon_policy_state(&policy_path(state_paths))
}

fn read_addon_policy_state(path: &Path) -> AppResult<AddonPolicyState> {
    if !path.exists() {
        return Ok(AddonPolicyState::default());
    }

    let content = fs::read_to_string(path)?;
    let state = toml::from_str::<AddonPolicyState>(&content)?;
    validate_addon_policy_state(&state)?;
    Ok(state)
}

fn persist_addon_policy_state(path: &Path, state: &mut AddonPolicyState) -> AppResult<()> {
    if state.packages.is_empty() {
        cleanup_addon_policy(path)?;
        return Ok(());
    }

    state.updated_at = now_rfc3339()?;
    state
        .packages
        .sort_by(|left, right| left.package_id.cmp(&right.package_id));
    validate_addon_policy_state(state)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    write_bytes_atomically(path, toml::to_string_pretty(state)?.as_bytes())?;
    Ok(())
}

fn cleanup_addon_policy(path: &Path) -> AppResult<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }

    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if !parent.exists() {
        return Ok(());
    }

    let mut entries = fs::read_dir(parent)?;
    if entries.next().is_none() {
        fs::remove_dir(parent)?;
    }

    Ok(())
}

fn validate_set_request(request: &SetAddonPolicyRequest) -> AppResult<()> {
    if request.package.trim().is_empty() {
        return Err(AppError::Validation(
            "addon policy package selector cannot be empty".to_string(),
        ));
    }
    if !request.has_updates() {
        return Err(AppError::Validation(
            "no addon policy updates were provided".to_string(),
        ));
    }
    build_requested_pin(request)?;
    Ok(())
}

fn build_requested_pin(request: &SetAddonPolicyRequest) -> AppResult<Option<AddonPolicyPin>> {
    match (
        request.pinned_version.as_deref().map(str::trim),
        request.pinned_file_id,
    ) {
        (Some(_version), Some(_)) => Err(AppError::Validation(
            "addon policy cannot pin both a version and a file id in one entry".to_string(),
        )),
        (Some(version), None) => {
            if version.is_empty() {
                return Err(AppError::Validation(
                    "addon policy pinned version cannot be empty".to_string(),
                ));
            }
            Ok(Some(AddonPolicyPin::Version {
                value: version.to_string(),
            }))
        }
        (None, Some(file_id)) => {
            if file_id == 0 {
                return Err(AppError::Validation(
                    "addon policy pinned file id must be greater than zero".to_string(),
                ));
            }
            Ok(Some(AddonPolicyPin::FileId { value: file_id }))
        }
        (None, None) => Ok(None),
    }
}

fn resolve_policy_target(
    tracked_packages: &[TrackedAddonPackage],
    state: &AddonPolicyState,
    package: &str,
) -> AppResult<(String, Option<TrackedAddonPackage>)> {
    let registry = crate::core::addon::AddonRegistry {
        schema_version: 1,
        packages: tracked_packages.to_vec(),
    };
    match select_tracked_packages(&registry, Some(package)) {
        Ok(mut matches) => {
            if matches.len() > 1 {
                let package_ids = matches
                    .iter()
                    .map(|candidate| candidate.package_id.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(AppError::Validation(format!(
                    "addon policy selector `{package}` matched multiple tracked packages: {package_ids}"
                )));
            }
            let tracked_package = matches.drain(..).next().ok_or_else(|| {
                AppError::NotFound(format!("no tracked addon package matched `{package}`"))
            })?;
            return Ok((tracked_package.package_id.clone(), Some(tracked_package)));
        }
        Err(AppError::NotFound(_)) => {}
        Err(error) => return Err(error),
    }

    if let Some(entry) = state
        .packages
        .iter()
        .find(|entry| entry.package_id.eq_ignore_ascii_case(package))
    {
        let tracked_package = tracked_packages
            .iter()
            .find(|candidate| candidate.package_id.eq_ignore_ascii_case(&entry.package_id))
            .cloned();
        return Ok((entry.package_id.clone(), tracked_package));
    }

    Err(AppError::NotFound(format!(
        "no tracked addon package or existing addon policy entry matched `{package}`"
    )))
}

fn validate_addon_policy_state(state: &AddonPolicyState) -> AppResult<()> {
    if state.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon policy schema version: {}",
            state.schema_version
        )));
    }
    if state.updated_at.trim().is_empty() {
        return Err(AppError::Validation(
            "addon policy updated_at must not be empty".to_string(),
        ));
    }

    let mut package_ids = BTreeSet::new();
    for entry in &state.packages {
        validate_addon_policy_entry(entry)?;
        let normalized = normalize_package_key(&entry.package_id);
        if !package_ids.insert(normalized) {
            return Err(AppError::Validation(format!(
                "duplicate addon policy package id: {}",
                entry.package_id
            )));
        }
    }

    Ok(())
}

fn validate_addon_policy_entry(entry: &AddonPolicyPackageEntry) -> AppResult<()> {
    if entry.package_id.trim().is_empty() {
        return Err(AppError::Validation(
            "addon policy package id cannot be empty".to_string(),
        ));
    }
    if !entry_has_policy_setting(entry) {
        return Err(AppError::Validation(format!(
            "addon policy package `{}` must contain at least one policy setting",
            entry.package_id
        )));
    }

    match &entry.pin {
        Some(AddonPolicyPin::Version { value }) if value.trim().is_empty() => {
            Err(AppError::Validation(format!(
                "addon policy pinned version cannot be empty for package `{}`",
                entry.package_id
            )))
        }
        Some(AddonPolicyPin::FileId { value }) if *value == 0 => {
            Err(AppError::Validation(format!(
                "addon policy pinned file id must be greater than zero for package `{}`",
                entry.package_id
            )))
        }
        _ => Ok(()),
    }
}

fn entry_has_policy_setting(entry: &AddonPolicyPackageEntry) -> bool {
    entry.ignored.is_some()
        || entry.pin.is_some()
        || entry.release_channel.is_some()
        || entry.allow_prerelease.is_some()
        || entry.install_dependencies.is_some()
}

fn now_rfc3339() -> AppResult<String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| AppError::Validation(error.to_string()))
}
