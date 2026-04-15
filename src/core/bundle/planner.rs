use std::fs::File;
use std::path::Path;

use zip::ZipArchive;

use super::*;

struct BundleReader<'a> {
    bundle_path: &'a Path,
}

struct BundleReadModel {
    inspection: BundleInspection,
    entry_names: Vec<String>,
}

struct BundlePlanner<'a> {
    bundle_path: &'a Path,
    installation: &'a DetectedFlavorInstallation,
    apply_mappings: &'a BundleApplyMappings,
}

pub fn plan_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<BundleApplyPlan> {
    Ok(prepare_bundle_apply(bundle_path, installation, apply_mappings)?.plan)
}

pub(super) fn prepare_bundle_apply(
    bundle_path: &Path,
    installation: &DetectedFlavorInstallation,
    apply_mappings: &BundleApplyMappings,
) -> AppResult<PreparedBundleApply> {
    BundlePlanner {
        bundle_path,
        installation,
        apply_mappings,
    }
    .prepare()
}

impl<'a> BundleReader<'a> {
    fn new(bundle_path: &'a Path) -> Self {
        Self { bundle_path }
    }

    fn inspect(&self) -> AppResult<BundleInspection> {
        inspect_bundle(self.bundle_path)
    }

    fn read_for_apply(&self) -> AppResult<BundleReadModel> {
        let inspection = self.inspect()?;

        Ok(BundleReadModel {
            inspection,
            entry_names: collect_bundle_entry_names(self.bundle_path)?,
        })
    }
}

impl<'a> BundlePlanner<'a> {
    fn prepare(&self) -> AppResult<PreparedBundleApply> {
        let read_model = BundleReader::new(self.bundle_path).read_for_apply()?;
        validate_target_compatibility(&read_model.inspection.manifest, self.installation)?;
        let discovered_accounts = discover_local_accounts(self.installation)?;
        let character_mappings =
            build_character_mappings(&read_model.inspection.manifest, self.apply_mappings)?;
        let selected_target_accounts = resolve_selected_target_accounts(
            &read_model.inspection.manifest,
            &discovered_accounts,
            &character_mappings,
            self.apply_mappings,
        )?;
        self.plan(
            read_model,
            discovered_accounts,
            character_mappings,
            selected_target_accounts,
        )
    }

    fn plan(
        &self,
        read_model: BundleReadModel,
        discovered_accounts: Vec<LocalWowAccount>,
        character_mappings: Vec<CharacterMapping>,
        selected_target_accounts: Vec<String>,
    ) -> AppResult<PreparedBundleApply> {
        let inspection = read_model.inspection;
        let planned_entries = plan_extractable_entries(
            &read_model.entry_names,
            self.installation,
            &inspection.manifest,
            &character_mappings,
            self.apply_mappings,
            &selected_target_accounts,
        )?;
        let file = File::open(self.bundle_path)?;
        let mut archive = ZipArchive::new(file)?;
        let rewrite_options = LuaRewriteOptions {
            rewrite_profile_keys: inspection.manifest.mapping.rewrite_profile_keys,
            rewrite_identity_strings: inspection.manifest.mapping.rewrite_identity_strings,
        };

        let cleanup_operations =
            build_cleanup_operations(&planned_entries, &inspection.manifest, self.installation)?;
        let cleanup_roots = cleanup_operations
            .iter()
            .map(|operation| operation.destination.clone())
            .collect::<Vec<_>>();
        let mut execution_operations = Vec::new();
        let mut summary = ApplyPlanSummary::default();
        for cleanup in cleanup_operations {
            summary.paths_to_remove += 1;
            execution_operations.push(PreparedApplyOperation::from_cleanup(cleanup));
        }

        for entry in &planned_entries {
            let policy = resource_policy_for_group(&inspection.manifest, entry.group);
            let preserve = policy == ResourceApplyPolicy::Preserve;
            let share = policy == ResourceApplyPolicy::Share;
            let cleanup_root = cleanup_scope_for_entry(entry, self.installation)?;
            let will_cleanup = cleanup_root
                .as_ref()
                .is_some_and(|root| cleanup_roots.iter().any(|candidate| candidate == root));
            let source_bytes =
                read_bundle_entry_bytes_from_archive(&mut archive, &entry.archive_name)?;
            let rewritten_bytes = if preserve {
                None
            } else {
                preview_lua_bytes_rewrite(
                    Path::new(&entry.archive_name),
                    &source_bytes,
                    &entry.rewrites,
                    rewrite_options,
                )?
            };
            let rewrite_applied = rewritten_bytes.is_some();
            let action = if preserve {
                summary.files_to_preserve += 1;
                ApplyAction::Preserve
            } else if share && entry.destination.exists() {
                summary.files_to_preserve += 1;
                ApplyAction::Preserve
            } else if will_cleanup {
                summary.files_to_add += 1;
                ApplyAction::Add
            } else if !entry.destination.exists() {
                summary.files_to_add += 1;
                ApplyAction::Add
            } else if rewritten_bytes.as_deref().map_or_else(
                || file_contents_equal_to_bytes(&source_bytes, &entry.destination),
                |bytes| file_contents_equal_to_bytes(bytes, &entry.destination),
            )? {
                summary.files_to_skip += 1;
                ApplyAction::Skip
            } else {
                summary.files_to_replace += 1;
                ApplyAction::Replace
            };
            if rewrite_applied {
                summary.files_to_rewrite += 1;
            }

            execution_operations.push(PreparedApplyOperation::from_entry(
                entry,
                action,
                rewrite_applied,
            ));
        }

        execution_operations.sort_by(|left, right| {
            apply_action_order(left.action)
                .cmp(&apply_action_order(right.action))
                .then_with(|| apply_group_order(left.group).cmp(&apply_group_order(right.group)))
                .then_with(|| left.destination.cmp(&right.destination))
                .then_with(|| left.archive_name.cmp(&right.archive_name))
        });
        let operations = execution_operations
            .iter()
            .map(PreparedApplyOperation::preview)
            .collect::<Vec<_>>();

        Ok(PreparedBundleApply {
            plan: BundleApplyPlan {
                bundle_path: self.bundle_path.to_path_buf(),
                target_flavor_root: self.installation.flavor_root.clone(),
                discovered_accounts,
                selected_target_accounts,
                character_mappings,
                operations,
                summary,
                helper_strategy: HelperStrategy::NativeRust,
                group_policies: ApplyGroupPolicies {
                    addons: GroupPolicy {
                        policy: inspection.manifest.apply.addons,
                    },
                    wtf_common: GroupPolicy {
                        policy: inspection.manifest.apply.wtf_common,
                    },
                    wtf_characters: GroupPolicy {
                        policy: inspection.manifest.apply.wtf_characters,
                    },
                    fonts: GroupPolicy {
                        policy: inspection.manifest.apply.fonts,
                    },
                    interface_assets: GroupPolicy {
                        policy: inspection.manifest.apply.interface_assets,
                    },
                    metadata: GroupPolicy {
                        policy: ResourceApplyPolicy::Merge,
                    },
                },
                manifest: inspection.manifest,
            },
            execution_operations,
        })
    }
}
