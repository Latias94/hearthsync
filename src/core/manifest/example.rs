use crate::core::error::AppResult;
use crate::core::install::{HostPlatform, WowFlavor};

use super::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};

pub fn example_manifest() -> AppResult<String> {
    toml::to_string_pretty(&BundleManifest {
        schema_version: 1,
        package: PackageMetadata {
            id: "starter-ui-retail".to_string(),
            name: "Starter UI for Retail".to_string(),
            created_by: "hearthsync".to_string(),
            description: Some("Example bundle manifest for addon and config sync.".to_string()),
        },
        source: SourceInstallation {
            flavor: WowFlavor::Retail,
            platform: Some(HostPlatform::Windows),
            exported_at: Some("2026-04-15T00:00:00Z".to_string()),
            supported_targets: vec![WowFlavor::Retail],
        },
        resources: BundleResources {
            addons: vec![
                "WeakAuras".to_string(),
                "Plater".to_string(),
                "Details".to_string(),
            ],
            wtf_common: true,
            wtf_characters: vec![CharacterResource {
                source_account: Some("ACCOUNT_NAME".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Examplemage".to_string(),
                target_hint: Some("Map to your main character".to_string()),
            }],
            fonts: false,
            interface_assets: vec!["SharedXML".to_string()],
            addon_lock: false,
            addon_indexes: Vec::new(),
        },
        mapping: MappingRules {
            character_mode: CharacterMappingMode::Prompt,
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
            allow_cross_platform: true,
        },
        apply: ApplyDefaults {
            create_backup: true,
            addons: ResourceApplyPolicy::Merge,
            wtf_common: ResourceApplyPolicy::Merge,
            wtf_characters: ResourceApplyPolicy::Merge,
            fonts: ResourceApplyPolicy::Merge,
            interface_assets: ResourceApplyPolicy::Merge,
        },
    })
    .map_err(Into::into)
}
