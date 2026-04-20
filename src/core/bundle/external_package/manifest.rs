use super::{CreateExternalPackageBundleRequest, ExternalPackageAnalysis};
use crate::core::manifest::{
    ApplyDefaults, BundleManifest, CharacterMappingMode, MappingRules, PackageMetadata,
    ResourceApplyPolicy, SourceInstallation,
};

pub(super) fn build_external_manifest(
    analysis: &ExternalPackageAnalysis,
    request: &CreateExternalPackageBundleRequest,
) -> BundleManifest {
    let package_id = request
        .package_id
        .clone()
        .unwrap_or_else(|| analysis.package_id.clone());
    let package_name = request
        .package_name
        .clone()
        .unwrap_or_else(|| analysis.package_name.clone());
    let created_by = request
        .created_by
        .clone()
        .unwrap_or_else(|| "external-package".to_string());
    let description = request.description.clone().or_else(|| {
        Some(format!(
            "Normalized import bundle created from external package `{}`.",
            analysis.source_path.display()
        ))
    });
    let supported_targets = if request.supported_targets.is_empty() {
        vec![request.source_flavor]
    } else {
        request.supported_targets.clone()
    };
    let character_mode = if analysis.resources.wtf_characters.is_empty() {
        CharacterMappingMode::KeepOriginal
    } else {
        CharacterMappingMode::Prompt
    };

    BundleManifest {
        schema_version: 1,
        package: PackageMetadata {
            id: package_id,
            name: package_name,
            created_by,
            description,
        },
        source: SourceInstallation {
            flavor: request.source_flavor,
            platform: request.source_platform,
            exported_at: None,
            supported_targets,
        },
        resources: analysis.resources.clone(),
        mapping: MappingRules {
            character_mode,
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
            allow_cross_platform: true,
        },
        apply: request
            .apply_defaults
            .clone()
            .unwrap_or_else(author_package_apply_defaults),
    }
}

pub(crate) fn author_package_apply_defaults() -> ApplyDefaults {
    ApplyDefaults {
        create_backup: true,
        addons: ResourceApplyPolicy::Mirror,
        wtf_common: ResourceApplyPolicy::Share,
        wtf_characters: ResourceApplyPolicy::ReplaceSelected,
        fonts: ResourceApplyPolicy::Mirror,
        interface_assets: ResourceApplyPolicy::Mirror,
    }
}
