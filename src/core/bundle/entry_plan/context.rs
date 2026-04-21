use super::super::apply_model::planned::PlannedEntry;
use super::super::entry_layout::{BundleArchiveEntry, classify_bundle_archive_entry};
use super::super::shared::path::{join_segments, safe_file_part};
use super::super::target_accounts::common::resolve_common_account_targets;
use super::super::types::apply::BundleApplyMappings;
use super::EntryPlanningContext;
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;
use crate::core::lua_patch::CharacterMapping;
use crate::core::manifest::BundleManifest;

pub(in crate::core::bundle) fn plan_extractable_entries(
    entry_names: &[String],
    installation: &DetectedFlavorInstallation,
    manifest: &BundleManifest,
    character_mappings: &[CharacterMapping],
    apply_mappings: &BundleApplyMappings,
    selected_target_accounts: &[String],
) -> AppResult<Vec<PlannedEntry>> {
    let common_account_targets = resolve_common_account_targets(
        manifest,
        character_mappings,
        apply_mappings,
        selected_target_accounts,
    )?;
    let context = EntryPlanningContext {
        installation,
        manifest,
        character_mappings,
        common_account_targets: &common_account_targets,
        default_target_account: apply_mappings.target_account.as_deref(),
        selected_target_accounts,
    };
    let mut planned_entries = Vec::new();

    for archive_name in entry_names {
        planned_entries.extend(context.plan_entry(archive_name)?);
    }

    Ok(planned_entries)
}

impl<'a> EntryPlanningContext<'a> {
    fn plan_entry(&self, archive_name: &str) -> AppResult<Vec<PlannedEntry>> {
        let Some(entry) = classify_bundle_archive_entry(archive_name)? else {
            return Ok(Vec::new());
        };

        self.plan_classified_entry(archive_name, entry)
    }

    fn plan_classified_entry(
        &self,
        archive_name: &str,
        entry: BundleArchiveEntry<'_>,
    ) -> AppResult<Vec<PlannedEntry>> {
        let metadata = entry.metadata();

        match entry {
            BundleArchiveEntry::Metadata { rest } => Ok(vec![
                self.simple_entry(
                    archive_name,
                    join_segments(
                        &self
                            .installation
                            .addon_dir
                            .join(".hearthsync")
                            .join("bundles")
                            .join(safe_file_part(&self.manifest.package.id)),
                        &rest,
                    ),
                    metadata.group,
                    metadata.wtf_scope,
                ),
            ]),
            BundleArchiveEntry::Addon { rest } => Ok(vec![self.simple_entry(
                archive_name,
                join_segments(&self.installation.addon_dir, &rest),
                metadata.group,
                metadata.wtf_scope,
            )]),
            BundleArchiveEntry::CommonConfig => Ok(vec![self.simple_entry(
                archive_name,
                self.installation.wtf_dir.join("Config.wtf"),
                metadata.group,
                metadata.wtf_scope,
            )]),
            BundleArchiveEntry::CommonRootSavedVariables { rest } => Ok(vec![
                self.plan_root_saved_variables_entry(archive_name, &rest),
            ]),
            BundleArchiveEntry::CommonAccountSavedVariables {
                source_account,
                rest,
            } => Ok(self.plan_common_account_entries(
                archive_name,
                source_account,
                &["SavedVariables"],
                &rest,
                metadata
                    .wtf_scope
                    .expect("common account saved variables entries always carry a WTF scope"),
            )),
            BundleArchiveEntry::CommonAccountFile {
                source_account,
                rest,
            } => Ok(self.plan_common_account_entries(
                archive_name,
                source_account,
                &[],
                &rest,
                metadata
                    .wtf_scope
                    .expect("common account file entries always carry a WTF scope"),
            )),
            BundleArchiveEntry::CharacterFile {
                source_account,
                server,
                character,
                rest,
            } => Ok(vec![
                self.plan_character_entry(
                    archive_name,
                    source_account,
                    server,
                    character,
                    &rest,
                    metadata
                        .wtf_scope
                        .expect("character file entries always carry a WTF scope"),
                ),
            ]),
            BundleArchiveEntry::Fonts { rest } => Ok(vec![self.simple_entry(
                archive_name,
                join_segments(&self.installation.fonts_dir, &rest),
                metadata.group,
                metadata.wtf_scope,
            )]),
            BundleArchiveEntry::Interface { rest } => Ok(vec![self.simple_entry(
                archive_name,
                join_segments(&self.installation.interface_dir, &rest),
                metadata.group,
                metadata.wtf_scope,
            )]),
        }
    }
}
