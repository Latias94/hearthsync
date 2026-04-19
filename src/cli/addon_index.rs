use super::AddonIndexCommands;
use super::output::render;
use crate::core::app::{
    HearthSyncApp, InspectAddonIndexRequest, InstallAddonIndexAppRequest,
    ResolveInstallationRequest, UpdateAddonIndexAppRequest,
};
use crate::core::error::AppResult;

pub(super) fn handle_addon_index_command(json: bool, command: AddonIndexCommands) -> AppResult<()> {
    let app = HearthSyncApp::new();

    match command {
        AddonIndexCommands::Inspect { file } => {
            let inspection =
                app.inspect_addon_index(InspectAddonIndexRequest { index_path: file })?;
            render(json, &inspection, |item| {
                let packages = item
                    .packages
                    .iter()
                    .map(|package| {
                        format!(
                            "{} {} => {}",
                            package.id, package.version, package.source_label
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "Index: {}\nName: {}\nPackages: {}\n{}",
                    item.index_path.display(),
                    item.name,
                    item.package_count,
                    if packages.is_empty() {
                        "none".to_string()
                    } else {
                        packages
                    }
                )
            })?;
        }
        AddonIndexCommands::Install {
            install,
            flavor,
            file,
            name,
            dry_run,
            backup_output,
            replace_existing,
        } => {
            let installation = app
                .stable()
                .resolve_installation(ResolveInstallationRequest {
                    path: install,
                    flavor: flavor.map(Into::into),
                })?;
            let result = app.install_addon_index(InstallAddonIndexAppRequest {
                installation,
                index_path: file,
                name,
                dry_run,
                backup_output_path: backup_output,
                replace_existing,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .install
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let addons = item
                    .install
                    .addons
                    .iter()
                    .map(|addon| addon.directory_name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                if item.install.dry_run {
                    format!(
                        "Dry run only.\nIndex: {}\nPackage: {} {}\nAddons: {}\nFiles to write: {}\nBackup: {}",
                        item.index_path.display(),
                        item.package.id,
                        item.package.version,
                        addons,
                        item.install.files_to_write,
                        backup
                    )
                } else {
                    format!(
                        "Installed index package: {} {}\nIndex: {}\nAddons: {}\nWritten files: {}\nBackup: {}",
                        item.package.id,
                        item.package.version,
                        item.index_path.display(),
                        addons,
                        item.install.written_files,
                        backup
                    )
                }
            })?;
        }
        AddonIndexCommands::Update {
            install,
            flavor,
            file,
            name,
            dry_run,
            backup_output,
        } => {
            let installation = app
                .stable()
                .resolve_installation(ResolveInstallationRequest {
                    path: install,
                    flavor: flavor.map(Into::into),
                })?;
            let result = app.update_addon_index(UpdateAddonIndexAppRequest {
                installation,
                index_path: file,
                name,
                dry_run,
                backup_output_path: backup_output,
            })?;
            render(json, &result, |item| {
                let backup = item
                    .update
                    .backup_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_string());
                let packages = item
                    .selected_packages
                    .iter()
                    .map(|package| format!("{} {}", package.id, package.version))
                    .collect::<Vec<_>>()
                    .join(", ");
                if item.update.dry_run {
                    format!(
                        "Dry run only.\nIndex: {}\nPackages: {}\nFiles to write: {}\nBackup: {}",
                        item.index_path.display(),
                        packages,
                        item.update.files_to_write,
                        backup
                    )
                } else {
                    format!(
                        "Updated index packages: {}\nIndex: {}\nWritten files: {}\nBackup: {}",
                        packages,
                        item.index_path.display(),
                        item.update.written_files,
                        backup
                    )
                }
            })?;
        }
    }

    Ok(())
}
