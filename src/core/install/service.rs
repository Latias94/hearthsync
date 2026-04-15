use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::core::error::{AppError, AppResult};

use super::model::{
    DetectedFlavorInstallation, HealthStatus, HostPlatform, InstallationHealth, LocalWowAccount,
    LocalWowCharacter, ProductInstallInspection, WowFlavor,
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

pub fn discover_local_accounts(
    installation: &DetectedFlavorInstallation,
) -> AppResult<Vec<LocalWowAccount>> {
    let account_root = installation.wtf_dir.join("Account");
    if !account_root.exists() {
        return Ok(Vec::new());
    }

    let mut accounts = Vec::new();
    for entry in std::fs::read_dir(&account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let account_name = entry.file_name().to_string_lossy().to_string();
        if is_reserved_account_entry(&account_name) {
            continue;
        }

        let saved_variables_dir = account_dir.join("SavedVariables");
        let mut characters = Vec::new();

        for server_entry in std::fs::read_dir(&account_dir)? {
            let server_entry = server_entry?;
            let server_dir = server_entry.path();
            if !server_dir.is_dir() {
                continue;
            }

            let server_name = server_entry.file_name().to_string_lossy().to_string();
            if server_name.eq_ignore_ascii_case("SavedVariables") {
                continue;
            }

            for character_entry in std::fs::read_dir(&server_dir)? {
                let character_entry = character_entry?;
                let character_dir = character_entry.path();
                if !character_dir.is_dir() {
                    continue;
                }

                characters.push(LocalWowCharacter {
                    server: server_name.clone(),
                    character: character_entry.file_name().to_string_lossy().to_string(),
                    character_dir,
                });
            }
        }

        characters.sort_by(|left, right| {
            left.server
                .cmp(&right.server)
                .then(left.character.cmp(&right.character))
        });

        accounts.push(LocalWowAccount {
            account_name,
            account_dir,
            saved_variables_dir,
            characters,
        });
    }

    accounts.sort_by(|left, right| left.account_name.cmp(&right.account_name));
    Ok(accounts)
}

fn is_reserved_account_entry(name: &str) -> bool {
    name.eq_ignore_ascii_case("SavedVariables")
}

fn candidate_product_roots() -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();

    if let Ok(custom_root) = std::env::var("WOW_INSTALL_ROOT") {
        roots.insert(PathBuf::from(custom_root));
    }

    if let Some(base_dirs) = BaseDirs::new() {
        roots.extend(common_roots_for_platform(base_dirs.home_dir()));
    }

    roots.into_iter().collect()
}

fn common_roots_for_platform(home_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = BTreeSet::new();

    match HostPlatform::current() {
        HostPlatform::Windows => {
            for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
                if let Ok(value) = std::env::var(variable) {
                    candidates.insert(PathBuf::from(value).join("World of Warcraft"));
                }
            }

            candidates.insert(home_dir.join("Games").join("World of Warcraft"));

            for drive in 'C'..='Z' {
                let root = PathBuf::from(format!("{drive}:\\"));
                candidates.insert(root.join("World of Warcraft"));
                candidates.insert(root.join("Games").join("World of Warcraft"));
                candidates.insert(root.join("Blizzard").join("World of Warcraft"));
                candidates.insert(root.join("Program Files").join("World of Warcraft"));
                candidates.insert(root.join("Program Files (x86)").join("World of Warcraft"));
            }
        }
        HostPlatform::MacOs => {
            candidates.insert(PathBuf::from("/Applications/World of Warcraft"));
            candidates.insert(home_dir.join("Applications").join("World of Warcraft"));
            candidates.insert(home_dir.join("Games").join("World of Warcraft"));
        }
        HostPlatform::Linux | HostPlatform::Unknown => {
            candidates.insert(home_dir.join("Games").join("World of Warcraft"));
        }
    }

    candidates.into_iter().collect()
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

fn build_installation(
    product_root: &Path,
    flavor_root: &Path,
    flavor: WowFlavor,
) -> DetectedFlavorInstallation {
    let interface_dir = flavor_root.join("Interface");
    let addon_dir = interface_dir.join("AddOns");
    let wtf_dir = flavor_root.join("WTF");
    let fonts_dir = flavor_root.join("Fonts");

    DetectedFlavorInstallation {
        platform: HostPlatform::current(),
        product_root: product_root.to_path_buf(),
        flavor_root: flavor_root.to_path_buf(),
        flavor,
        interface_dir,
        addon_dir,
        wtf_dir,
        fonts_dir,
    }
}

fn detect_flavors(product_root: &Path) -> Vec<WowFlavor> {
    let mut flavors = Vec::new();

    for flavor in [
        WowFlavor::Retail,
        WowFlavor::Classic,
        WowFlavor::ClassicEra,
        WowFlavor::Ptr,
        WowFlavor::Beta,
        WowFlavor::Xptr,
    ] {
        let candidate = product_root.join(flavor.folder_name());
        if has_wow_structure(&candidate) {
            flavors.push(flavor);
        }
    }

    flavors
}

fn has_wow_structure(path: &Path) -> bool {
    path.join("Interface").is_dir() && path.join("WTF").is_dir()
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

fn normalize_path(path: &Path) -> AppResult<PathBuf> {
    if !path.exists() {
        return Err(AppError::NotFound(format!(
            "Path does not exist: {}",
            path.display()
        )));
    }

    Ok(strip_windows_verbatim_prefix(
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
    ))
}

fn strip_windows_verbatim_prefix(path: PathBuf) -> PathBuf {
    let raw = path.to_string_lossy();
    if let Some(stripped) = raw.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::{HealthStatus, discover_local_accounts, inspect_installation};
    use crate::core::install::WowFlavor;

    #[test]
    fn inspect_installation_resolves_product_root() {
        let temp = tempdir().expect("temp dir");
        let product_root = temp.path().join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");

        fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
        fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");
        fs::write(
            flavor_root.join("WTF").join("Config.wtf"),
            "SET locale enUS",
        )
        .expect("config");

        let inspection =
            inspect_installation(&product_root, Some(WowFlavor::Retail)).expect("inspect");

        assert_eq!(inspection.installation.flavor, WowFlavor::Retail);
        assert_eq!(inspection.health.status, HealthStatus::Warning);
        assert!(
            inspection
                .installation
                .flavor_root
                .ends_with(Path::new("World of Warcraft").join("_retail_"))
        );
    }

    #[test]
    fn discover_local_accounts_reads_accounts_and_characters() {
        let temp = tempdir().expect("temp dir");
        let product_root = temp.path().join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");

        fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
        fs::create_dir_all(
            flavor_root
                .join("WTF")
                .join("Account")
                .join("SavedVariables"),
        )
        .expect("global saved variables");
        fs::create_dir_all(
            flavor_root
                .join("WTF")
                .join("Account")
                .join("ACC1")
                .join("SavedVariables"),
        )
        .expect("saved variables");
        fs::create_dir_all(
            flavor_root
                .join("WTF")
                .join("Account")
                .join("ACC1")
                .join("Illidan")
                .join("Mageone"),
        )
        .expect("character");
        fs::write(
            flavor_root.join("WTF").join("Config.wtf"),
            "SET locale enUS",
        )
        .expect("config");

        let installation =
            inspect_installation(&product_root, Some(WowFlavor::Retail)).expect("inspect");
        let accounts =
            discover_local_accounts(&installation.installation).expect("discover accounts");

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].account_name, "ACC1");
        assert!(
            accounts[0]
                .saved_variables_dir
                .ends_with(Path::new("ACC1").join("SavedVariables"))
        );
        assert_eq!(accounts[0].characters.len(), 1);
        assert_eq!(accounts[0].characters[0].server, "Illidan");
        assert_eq!(accounts[0].characters[0].character, "Mageone");
    }
}
