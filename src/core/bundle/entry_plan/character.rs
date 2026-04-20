use super::super::character_mapping_match::find_character_mapping;
use super::super::wtf_scope::classify_character_wtf_scope;
use super::*;

impl<'a> EntryPlanningContext<'a> {
    pub(in crate::core::bundle::entry_plan) fn plan_character_entry(
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
