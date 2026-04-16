use std::collections::BTreeMap;

use super::character_mapping::find_character_mapping;
use super::entry_layout::{BundleArchiveEntry, classify_bundle_archive_entry};
use super::target_accounts::resolve_common_account_targets;
use super::wtf_scope::{classify_account_wtf_scope, classify_character_wtf_scope};
use super::*;

pub(super) fn plan_extractable_entries(
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

struct EntryPlanningContext<'a> {
    installation: &'a DetectedFlavorInstallation,
    manifest: &'a BundleManifest,
    character_mappings: &'a [CharacterMapping],
    common_account_targets: &'a BTreeMap<String, String>,
    default_target_account: Option<&'a str>,
    selected_target_accounts: &'a [String],
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
                    ApplyGroup::Metadata,
                    None,
                ),
            ]),
            BundleArchiveEntry::Addon { rest } => Ok(vec![self.simple_entry(
                archive_name,
                join_segments(&self.installation.addon_dir, &rest),
                ApplyGroup::Addons,
                None,
            )]),
            BundleArchiveEntry::CommonConfig => Ok(vec![self.simple_entry(
                archive_name,
                self.installation.wtf_dir.join("Config.wtf"),
                ApplyGroup::WtfCommon,
                Some(WtfScope::GlobalConfig),
            )]),
            BundleArchiveEntry::CommonAccountSavedVariables {
                source_account,
                rest,
            } => Ok(self.plan_common_account_entries(
                archive_name,
                source_account,
                &["SavedVariables"],
                &rest,
                WtfScope::AccountSavedVariables,
            )),
            BundleArchiveEntry::CommonAccountFile {
                source_account,
                rest,
            } => Ok(self.plan_common_account_entries(
                archive_name,
                source_account,
                &[],
                &rest,
                classify_account_wtf_scope(&rest),
            )),
            BundleArchiveEntry::CharacterFile {
                source_account,
                server,
                character,
                rest,
            } => Ok(vec![self.plan_character_entry(
                archive_name,
                source_account,
                server,
                character,
                &rest,
            )]),
            BundleArchiveEntry::Fonts { rest } => Ok(vec![self.simple_entry(
                archive_name,
                join_segments(&self.installation.fonts_dir, &rest),
                ApplyGroup::Fonts,
                None,
            )]),
            BundleArchiveEntry::Interface { rest } => Ok(vec![self.simple_entry(
                archive_name,
                join_segments(&self.installation.interface_dir, &rest),
                ApplyGroup::InterfaceAssets,
                None,
            )]),
        }
    }

    fn simple_entry(
        &self,
        archive_name: &str,
        destination: PathBuf,
        group: ApplyGroup,
        wtf_scope: Option<WtfScope>,
    ) -> PlannedEntry {
        PlannedEntry {
            archive_name: archive_name.to_string(),
            destination,
            rewrites: Vec::new(),
            group,
            wtf_scope,
            target_account: None,
            target_server: None,
            target_character: None,
        }
    }

    fn plan_common_account_entries(
        &self,
        archive_name: &str,
        source_account: &str,
        prefix_segments: &[&str],
        rest: &[&str],
        wtf_scope: WtfScope,
    ) -> Vec<PlannedEntry> {
        self.resolve_common_target_accounts(source_account)
            .into_iter()
            .map(|target_account| PlannedEntry {
                archive_name: archive_name.to_string(),
                destination: self
                    .installation
                    .wtf_dir
                    .join("Account")
                    .join(&target_account)
                    .join(join_segments(Path::new(""), prefix_segments))
                    .join(join_segments(Path::new(""), rest)),
                rewrites: self.rewrites_for_target_account(&target_account),
                group: ApplyGroup::WtfCommon,
                wtf_scope: Some(wtf_scope),
                target_account: Some(target_account),
                target_server: None,
                target_character: None,
            })
            .collect()
    }

    fn resolve_common_target_accounts(&self, source_account: &str) -> Vec<String> {
        if !self.selected_target_accounts.is_empty() {
            self.selected_target_accounts.to_vec()
        } else {
            vec![
                self.common_account_targets
                    .get(source_account)
                    .cloned()
                    .or_else(|| self.default_target_account.map(|item| item.to_string()))
                    .unwrap_or_else(|| source_account.to_string()),
            ]
        }
    }

    fn rewrites_for_target_account(&self, target_account: &str) -> Vec<CharacterMapping> {
        self.character_mappings
            .iter()
            .filter(|mapping| mapping.target_account == target_account)
            .cloned()
            .collect()
    }

    fn plan_character_entry(
        &self,
        archive_name: &str,
        source_account: &str,
        server: &str,
        character: &str,
        rest: &[&str],
    ) -> PlannedEntry {
        let mapping = self
            .resolve_character_mapping(source_account, server, character)
            .clone();

        PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: self
                .installation
                .wtf_dir
                .join("Account")
                .join(&mapping.target_account)
                .join(&mapping.target_server)
                .join(&mapping.target_character)
                .join(join_segments(Path::new(""), rest)),
            rewrites: vec![mapping.clone()],
            group: ApplyGroup::WtfCharacters,
            wtf_scope: Some(classify_character_wtf_scope(rest)),
            target_account: Some(mapping.target_account),
            target_server: Some(mapping.target_server),
            target_character: Some(mapping.target_character),
        }
    }

    fn resolve_character_mapping(
        &self,
        source_account: &str,
        server: &str,
        character: &str,
    ) -> CharacterMapping {
        find_character_mapping(self.character_mappings, source_account, server, character)
            .cloned()
            .unwrap_or_else(|| CharacterMapping {
                source_account: Some(source_account.to_string()),
                source_server: server.to_string(),
                source_character: character.to_string(),
                target_account: source_account.to_string(),
                target_server: server.to_string(),
                target_character: character.to_string(),
            })
    }
}
