use crate::core::bundle::BundleApplyMappings;

pub(super) fn merge_apply_mapping_overrides(
    apply_mappings: &mut BundleApplyMappings,
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
