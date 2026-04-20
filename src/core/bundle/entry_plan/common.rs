use super::*;

impl<'a> EntryPlanningContext<'a> {
    pub(in crate::core::bundle::entry_plan) fn simple_entry(
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

    pub(in crate::core::bundle::entry_plan) fn plan_common_account_entries(
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

    pub(in crate::core::bundle::entry_plan) fn plan_root_saved_variables_entry(
        &self,
        archive_name: &str,
        rest: &[&str],
    ) -> PlannedEntry {
        PlannedEntry {
            archive_name: archive_name.to_string(),
            destination: self
                .installation
                .wtf_dir
                .join("Account")
                .join("SavedVariables")
                .join(join_segments(Path::new(""), rest)),
            rewrites: self.character_mappings.to_vec(),
            group: ApplyGroup::WtfCommon,
            wtf_scope: Some(WtfScope::RootSavedVariables),
            target_account: None,
            target_server: None,
            target_character: None,
        }
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
}
