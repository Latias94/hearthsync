use super::example_manifest;
use crate::core::install::WowFlavor;
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, MappingRules,
    PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};

#[test]
fn example_manifest_is_valid() {
    let manifest = example_manifest().expect("example");
    let parsed: BundleManifest = toml::from_str(&manifest).expect("parse");
    parsed.validate().expect("validate");
}

#[test]
fn prompt_character_mode_requires_character_resources() {
    let manifest = BundleManifest {
        schema_version: 1,
        package: PackageMetadata {
            id: "empty".to_string(),
            name: "Empty".to_string(),
            created_by: "test".to_string(),
            description: None,
        },
        source: SourceInstallation {
            flavor: WowFlavor::Retail,
            platform: None,
            exported_at: None,
            supported_targets: vec![WowFlavor::Retail],
        },
        resources: BundleResources {
            addons: vec!["WeakAuras".to_string()],
            wtf_common: false,
            wtf_characters: Vec::new(),
            fonts: false,
            interface_assets: Vec::new(),
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
    };

    let error = manifest
        .validate()
        .expect_err("prompt mode should require character resources");
    assert!(
        error
            .to_string()
            .contains("prompt character mapping requires")
    );
}
