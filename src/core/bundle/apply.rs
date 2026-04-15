use super::planner::prepare_bundle_apply;
use super::*;

pub fn unpack_bundle(request: UnpackBundleRequest) -> AppResult<UnpackedBundle> {
    let prepared = prepare_bundle_apply(
        &request.bundle_path,
        &request.installation,
        &request.apply_mappings,
    )?;
    let PreparedBundleApply {
        plan,
        execution_operations,
    } = prepared;
    if request.dry_run {
        return Ok(UnpackedBundle {
            bundle_path: request.bundle_path,
            target_flavor_root: request.installation.flavor_root,
            dry_run: true,
            planned_files: plan.operations.len(),
            written_files: 0,
            rewritten_files: 0,
            backup_path: None,
            selected_target_accounts: plan.selected_target_accounts,
            plan_summary: plan.summary,
            character_mappings: plan.character_mappings,
            manifest: plan.manifest,
        });
    }

    let execution = BundleExecutor {
        installation: &request.installation,
        backup_output_path: request.backup_output_path.clone(),
    }
    .execute(&plan, &execution_operations)?;

    Ok(UnpackedBundle {
        bundle_path: request.bundle_path,
        target_flavor_root: request.installation.flavor_root,
        dry_run: false,
        planned_files: plan.operations.len(),
        written_files: execution.written_files,
        rewritten_files: execution.rewritten_files,
        backup_path: execution.backup_path,
        selected_target_accounts: plan.selected_target_accounts,
        plan_summary: plan.summary,
        character_mappings: plan.character_mappings,
        manifest: plan.manifest,
    })
}

impl<'a> BundleExecutor<'a> {
    fn execute(
        &self,
        plan: &BundleApplyPlan,
        execution_operations: &[PreparedApplyOperation],
    ) -> AppResult<BundleExecution> {
        let backup_path = self.create_backup(plan)?;

        match execute_apply_operations(&plan.bundle_path, execution_operations, &plan.manifest) {
            Ok((written_files, rewritten_files)) => Ok(BundleExecution {
                backup_path,
                written_files,
                rewritten_files,
            }),
            Err(error) => {
                rollback_or_report_apply_error(error, backup_path.as_deref(), self.installation)
            }
        }
    }

    fn create_backup(&self, plan: &BundleApplyPlan) -> AppResult<Option<PathBuf>> {
        if !plan.manifest.apply.create_backup {
            return Ok(None);
        }

        let groups = backup_groups_for_manifest(&plan.manifest);
        if groups.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                create_backup(BackupRequest {
                    installation: self.installation.clone(),
                    output_path: self.backup_output_path.clone(),
                    groups,
                    label: Some("bundle-apply".to_string()),
                })?
                .archive_path,
            ))
        }
    }
}

fn backup_groups_for_manifest(manifest: &BundleManifest) -> Vec<BackupGroup> {
    let mut groups = Vec::new();

    if !manifest.resources.addons.is_empty()
        || manifest.resources.addon_lock
        || !manifest.resources.addon_indexes.is_empty()
    {
        groups.push(BackupGroup::Addons);
    }
    if manifest.resources.wtf_common || !manifest.resources.wtf_characters.is_empty() {
        groups.push(BackupGroup::Wtf);
    }
    if manifest.resources.fonts {
        groups.push(BackupGroup::Fonts);
    }
    if !manifest.resources.interface_assets.is_empty() {
        groups.push(BackupGroup::InterfaceAssets);
    }

    groups
}
