use crate::core::app::BundleApplyMappingsValue;
use crate::core::bundle::load_apply_mappings;
use crate::core::error::AppResult;

pub(super) fn merge_apply_mapping_overrides(
    apply_mappings: &mut BundleApplyMappingsValue,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
    selected_accounts: Vec<String>,
    all_accounts: bool,
) {
    if target_account.is_some() {
        apply_mappings.target_account = target_account;
    }
    if target_server.is_some() {
        apply_mappings.target_server = target_server;
    }
    if target_character.is_some() {
        apply_mappings.target_character = target_character;
    }
    if !selected_accounts.is_empty() {
        apply_mappings.selected_accounts = selected_accounts;
    }
    if all_accounts {
        apply_mappings.all_accounts = true;
    }
}

pub(super) fn resolve_apply_mappings(
    mapping_file: Option<&std::path::Path>,
    target_account: Option<String>,
    target_server: Option<String>,
    target_character: Option<String>,
    selected_accounts: Vec<String>,
    all_accounts: bool,
) -> AppResult<BundleApplyMappingsValue> {
    let mut apply_mappings = if let Some(path) = mapping_file {
        BundleApplyMappingsValue::from_domain(load_apply_mappings(path)?)
    } else {
        BundleApplyMappingsValue::default()
    };
    merge_apply_mapping_overrides(
        &mut apply_mappings,
        target_account,
        target_server,
        target_character,
        selected_accounts,
        all_accounts,
    );
    Ok(apply_mappings)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn resolve_apply_mappings_merges_file_and_cli_overrides() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mapping_file = temp_dir.path().join("mapping.toml");
        fs::write(
            &mapping_file,
            r#"
target_account = "FileAccount"
target_server = "FileServer"
target_character = "FileCharacter"
selected_accounts = ["FileA"]
all_accounts = false
"#,
        )
        .expect("write mapping file");

        let mappings = resolve_apply_mappings(
            Some(&mapping_file),
            Some("CliAccount".to_string()),
            None,
            Some("CliCharacter".to_string()),
            vec!["CliA".to_string(), "CliB".to_string()],
            true,
        )
        .expect("resolve mappings");

        assert_eq!(mappings.target_account.as_deref(), Some("CliAccount"));
        assert_eq!(mappings.target_server.as_deref(), Some("FileServer"));
        assert_eq!(mappings.target_character.as_deref(), Some("CliCharacter"));
        assert_eq!(
            mappings.selected_accounts,
            vec!["CliA".to_string(), "CliB".to_string()]
        );
        assert!(mappings.all_accounts);
        assert!(mappings.characters.is_empty());
    }
}
