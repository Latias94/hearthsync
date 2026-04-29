use super::example_manifest;
use crate::core::install::WowFlavor;
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};

#[test]
fn example_manifest_is_valid() {
    let manifest = example_manifest().expect("example");
    let parsed: BundleManifest = toml::from_str(&manifest).expect("parse");
    parsed.validate().expect("validate");
}

#[test]
fn prompt_character_mode_requires_character_resources() {
    let mut manifest = valid_manifest();
    manifest.resources.wtf_characters = Vec::new();
    manifest.mapping.character_mode = CharacterMappingMode::Prompt;

    let error = manifest
        .validate()
        .expect_err("prompt mode should require character resources");
    assert!(
        error
            .to_string()
            .contains("prompt character mapping requires")
    );
}

#[test]
fn manifest_rejects_empty_created_by() {
    let mut manifest = valid_manifest();
    manifest.package.created_by = " ".to_string();

    let error = manifest
        .validate()
        .expect_err("empty author should fail closed");

    assert!(
        error
            .to_string()
            .contains("package.created_by must not be empty")
    );
}

#[test]
fn manifest_rejects_non_portable_addon_resource() {
    let mut manifest = valid_manifest();
    manifest.resources.addons = vec!["Weak:Auras".to_string()];

    let error = manifest
        .validate()
        .expect_err("invalid addon resource should fail closed");

    assert!(error.to_string().contains("invalid resources.addons name"));
}

#[test]
fn manifest_rejects_non_portable_wtf_character_resource() {
    let mut manifest = valid_manifest();
    manifest.resources.wtf_characters[0].source_character = "Bad*Name".to_string();

    let error = manifest
        .validate()
        .expect_err("invalid character resource should fail closed");

    assert!(
        error
            .to_string()
            .contains("invalid resources.wtf_characters.source_character name")
    );
}

#[test]
fn manifest_rejects_non_portable_interface_asset_resource() {
    let mut manifest = valid_manifest();
    manifest.resources.interface_assets = vec!["Interface/Buttons".to_string()];

    let error = manifest
        .validate()
        .expect_err("invalid interface asset should fail closed");

    assert!(
        error
            .to_string()
            .contains("invalid resources.interface_assets name")
    );
}

fn valid_manifest() -> BundleManifest {
    BundleManifest {
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
            wtf_characters: vec![CharacterResource {
                source_account: Some("ACCOUNT".to_string()),
                source_server: "Illidan".to_string(),
                source_character: "Examplemage".to_string(),
                target_hint: None,
            }],
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
    }
}
