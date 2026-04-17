use super::AddonCommands;
use super::output::render;
use crate::core::addon::{
    InstallAddonRequest, RemoveAddonRequest, SearchAddonRequest, UpdateAddonRequest,
};
use crate::core::app::AddonService;
use crate::core::error::{AppError, AppResult};
use crate::core::install::resolve_installation;

pub(super) fn handle_basic_addon_command(json: bool, command: AddonCommands) -> AppResult<()> {
    let service = AddonService::new();

    match command {
        AddonCommands::Search {
            install,
            flavor,
            query,
            limit,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let results = service.search(SearchAddonRequest {
                installation,
                query,
                limit,
            })?;
            render(json, &results, |item| {
                if item.results.is_empty() {
                    format!("Query: {}\nNo addons found.", item.query)
                } else {
                    let mut lines = vec![
                        format!("Query: {}", item.query),
                        format!("Found {} result(s):", item.results.len()),
                    ];
                    for result in &item.results {
                        lines.push(format!(
                            "- {} | provider: {} | source: {} | downloads: {} | website: {}{}",
                            result.name,
                            result.provider,
                            result.install_hint,
                            result.download_count,
                            result.website_url.as_deref().unwrap_or("none"),
                            result
                                .summary
                                .as_deref()
                                .map(|summary| format!(" | summary: {summary}"))
                                .unwrap_or_default()
                        ));
                    }
                    lines.join("\n")
                }
            })?;
        }
        AddonCommands::List { install, flavor } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let inventory = service.list(&installation)?;
            render(json, &inventory, |item| {
                let tracked = if item.tracked_packages.is_empty() {
                    "none".to_string()
                } else {
                    item.tracked_packages
                        .iter()
                        .map(|package| {
                            format!(
                                "{} => {} [{}]",
                                package.package_id,
                                package.source.display_name(),
                                package
                                    .addons
                                    .iter()
                                    .map(|addon| addon.directory_name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                let untracked = if item.untracked_addons.is_empty() {
                    "none".to_string()
                } else {
                    item.untracked_addons.join(", ")
                };
                format!(
                    "Target: {}\nRegistry: {}\nTracked packages:\n{}\nUntracked addon directories: {}",
                    item.target_addon_root.display(),
                    item.registry_path.display(),
                    tracked,
                    untracked
                )
            })?;
        }
        AddonCommands::Install {
            install,
            flavor,
            source,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = service.install(InstallAddonRequest {
                installation,
                source,
                dry_run,
                backup_output_path: backup_output,
                replace_existing,
                metadata: None,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let replaced = if item.replaced_addons.is_empty() {
                    "none".to_string()
                } else {
                    item.replaced_addons.join(", ")
                };
                let addons = item
                    .addons
                    .iter()
                    .map(|addon| addon.directory_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                if item.dry_run {
                    format!(
                        "Dry run only.\nSource: {}\nPackage: {}\nAddons: {}\nFiles to write: {}\nWould replace: {}\nBackup: {}",
                        item.source.display_name(),
                        item.package_id,
                        addons,
                        item.files_to_write,
                        replaced,
                        backup
                    )
                } else {
                    format!(
                        "Installed package: {}\nSource: {}\nAddons: {}\nWritten files: {}\nReplaced addons: {}\nRegistry: {}\nBackup: {}",
                        item.package_id,
                        item.source.display_name(),
                        addons,
                        item.written_files,
                        replaced,
                        item.registry_path.display(),
                        backup
                    )
                }
            })?;
        }
        AddonCommands::Update {
            install,
            flavor,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = service.update(UpdateAddonRequest {
                installation,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let packages = if item.updated_packages.is_empty() {
                    "none".to_string()
                } else {
                    item.updated_packages
                        .iter()
                        .map(|package| {
                            format!(
                                "{} [{}]",
                                package.package_id,
                                package
                                    .addons
                                    .iter()
                                    .map(|addon| addon.directory_name.clone())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                if item.dry_run {
                    format!(
                        "Dry run only.\nRegistry: {}\nPackages: {}\nFiles to write: {}\nBackup: {}",
                        item.registry_path.display(),
                        packages,
                        item.files_to_write,
                        backup
                    )
                } else {
                    format!(
                        "Updated packages: {}\nWritten files: {}\nRegistry: {}\nBackup: {}",
                        packages,
                        item.written_files,
                        item.registry_path.display(),
                        backup
                    )
                }
            })?;
        }
        AddonCommands::Remove {
            install,
            flavor,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = resolve_installation(&install, flavor.map(Into::into))?;
            let result = service.remove(RemoveAddonRequest {
                installation,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let packages = if item.removed_packages.is_empty() {
                    "none".to_string()
                } else {
                    item.removed_packages
                        .iter()
                        .map(|package| package.package_id.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                let addons = if item.removed_addons.is_empty() {
                    "none".to_string()
                } else {
                    item.removed_addons.join(", ")
                };
                if item.dry_run {
                    format!(
                        "Dry run only.\nRegistry: {}\nPackages: {}\nAddon directories: {}\nBackup: {}",
                        item.registry_path.display(),
                        packages,
                        addons,
                        backup
                    )
                } else {
                    format!(
                        "Removed packages: {}\nRemoved addon directories: {}\nRegistry: {}\nRegistry cleaned: {}\nBackup: {}",
                        packages,
                        addons,
                        item.registry_path.display(),
                        item.registry_cleaned,
                        backup
                    )
                }
            })?;
        }
        AddonCommands::Index { .. } | AddonCommands::Lock { .. } => {
            return Err(AppError::Validation(
                "internal CLI routing error: addon subcommand reached basic addon handler"
                    .to_string(),
            ));
        }
    }

    Ok(())
}
