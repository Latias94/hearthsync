use std::path::PathBuf;

use super::{
    AddonPackageMetadataValue, BackupGroupValue, BundleApplyDefaultsValue,
    BundleApplyMappingsValue, BundleManifestValue, ResolvedInstallationValue,
};
use crate::core::addon::index::{
    AddonIndexInstallRequest as DomainAddonIndexInstallRequest,
    AddonIndexUpdateRequest as DomainAddonIndexUpdateRequest,
};
use crate::core::addon::lock::{
    AddonLockApplyRequest as DomainAddonLockApplyRequest,
    AddonLockSourceOverride as DomainAddonLockSourceOverride,
};
use crate::core::addon::{
    InstallAddonRequest as DomainInstallAddonRequest,
    RemoveAddonRequest as DomainRemoveAddonRequest, SearchAddonRequest as DomainSearchAddonRequest,
    UpdateAddonRequest as DomainUpdateAddonRequest,
};
use crate::core::backup::{
    BackupRequest as DomainBackupRequest, RestoreBackupRequest as DomainRestoreBackupRequest,
};
use crate::core::bundle::{
    AnalyzeExternalPackageRequest as DomainAnalyzeExternalPackageRequest,
    ApplyExternalPackageRequest as DomainApplyExternalPackageRequest,
    BundleAddonLockApplyRequest as DomainBundleAddonLockApplyRequest,
    CreateExternalPackageBundleRequest as DomainCreateExternalPackageBundleRequest,
    PackBundleRequest as DomainPackBundleRequest,
    PlanExternalPackageApplyRequest as DomainPlanExternalPackageApplyRequest,
    UnpackBundleRequest as DomainUnpackBundleRequest,
};
use crate::core::install::{HostPlatform, WowFlavor};

#[derive(Debug, Clone)]
pub struct SearchAddonsRequest {
    pub installation: ResolvedInstallationValue,
    pub query: String,
    pub limit: usize,
}

impl From<SearchAddonsRequest> for DomainSearchAddonRequest {
    fn from(request: SearchAddonsRequest) -> Self {
        Self {
            installation: request.installation.into(),
            query: request.query,
            limit: request.limit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListAddonsRequest {
    pub installation: ResolvedInstallationValue,
}

#[derive(Debug, Clone)]
pub struct InspectAddonIndexRequest {
    pub index_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct InspectAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

#[derive(Debug, Clone)]
pub struct WriteAddonLockRequest {
    pub installation: ResolvedInstallationValue,
}

#[derive(Debug, Clone)]
pub struct DiffAddonLockRequest {
    pub left_lock_path: PathBuf,
    pub right_lock_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct VerifyAddonLockRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct PlanAddonLockSyncRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct AddonLockSourceOverrideRequest {
    pub comparison_key: String,
    pub archive_path: PathBuf,
}

impl From<AddonLockSourceOverrideRequest> for DomainAddonLockSourceOverride {
    fn from(request: AddonLockSourceOverrideRequest) -> Self {
        Self {
            comparison_key: request.comparison_key,
            archive_path: request.archive_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplyAddonLockAppRequest {
    pub installation: ResolvedInstallationValue,
    pub lock_path: Option<PathBuf>,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub source_overrides: Vec<AddonLockSourceOverrideRequest>,
}

impl From<ApplyAddonLockAppRequest> for DomainAddonLockApplyRequest {
    fn from(request: ApplyAddonLockAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            lock_path: request.lock_path,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            source_overrides: request
                .source_overrides
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub source: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
    pub metadata: Option<AddonPackageMetadataValue>,
}

impl From<InstallAddonAppRequest> for DomainInstallAddonRequest {
    fn from(request: InstallAddonAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            source: request.source,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
            metadata: request.metadata.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl From<UpdateAddonAppRequest> for DomainUpdateAddonRequest {
    fn from(request: UpdateAddonAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoveAddonAppRequest {
    pub installation: ResolvedInstallationValue,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl From<RemoveAddonAppRequest> for DomainRemoveAddonRequest {
    fn from(request: RemoveAddonAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: String,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl From<InstallAddonIndexAppRequest> for DomainAddonIndexInstallRequest {
    fn from(request: InstallAddonIndexAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct UpdateAddonIndexAppRequest {
    pub installation: ResolvedInstallationValue,
    pub index_path: PathBuf,
    pub name: Option<String>,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
}

impl From<UpdateAddonIndexAppRequest> for DomainAddonIndexUpdateRequest {
    fn from(request: UpdateAddonIndexAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            index_path: request.index_path,
            name: request.name,
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ListBackupsRequest {
    pub backup_dir: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct CreateBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub output_path: Option<PathBuf>,
    pub groups: Vec<BackupGroupValue>,
    pub label: Option<String>,
}

impl From<CreateBackupAppRequest> for DomainBackupRequest {
    fn from(request: CreateBackupAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            output_path: request.output_path,
            groups: request.groups.into_iter().map(Into::into).collect(),
            label: request.label,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RestoreBackupAppRequest {
    pub installation: ResolvedInstallationValue,
    pub archive_path: Option<PathBuf>,
    pub backup_id: Option<String>,
    pub backup_dir: Option<PathBuf>,
}

impl From<RestoreBackupAppRequest> for DomainRestoreBackupRequest {
    fn from(request: RestoreBackupAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            archive_path: request.archive_path,
            backup_id: request.backup_id,
            backup_dir: request.backup_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InspectBundleRequest {
    pub bundle_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct PackBundleAppRequest {
    pub installation: ResolvedInstallationValue,
    pub manifest: BundleManifestValue,
    pub output_path: Option<PathBuf>,
    pub manifest_base_dir: Option<PathBuf>,
}

impl From<PackBundleAppRequest> for DomainPackBundleRequest {
    fn from(request: PackBundleAppRequest) -> Self {
        Self {
            installation: request.installation.into(),
            manifest: request.manifest.into(),
            output_path: request.output_path,
            manifest_base_dir: request.manifest_base_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanBundleApplyRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

#[derive(Debug, Clone)]
pub struct ApplyBundleAppRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl From<ApplyBundleAppRequest> for DomainUnpackBundleRequest {
    fn from(request: ApplyBundleAppRequest) -> Self {
        Self {
            bundle_path: request.bundle_path,
            installation: request.installation.into(),
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            apply_mappings: request.apply_mappings.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanBundleAddonLockRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
}

#[derive(Debug, Clone)]
pub struct ApplyBundleAddonLockAppRequest {
    pub bundle_path: PathBuf,
    pub installation: ResolvedInstallationValue,
    pub backup_output_path: Option<PathBuf>,
    pub replace_existing: bool,
}

impl From<ApplyBundleAddonLockAppRequest> for DomainBundleAddonLockApplyRequest {
    fn from(request: ApplyBundleAddonLockAppRequest) -> Self {
        Self {
            bundle_path: request.bundle_path,
            installation: request.installation.into(),
            backup_output_path: request.backup_output_path,
            replace_existing: request.replace_existing,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnalyzeExternalPackageAppRequest {
    pub source_path: PathBuf,
}

impl From<AnalyzeExternalPackageAppRequest> for DomainAnalyzeExternalPackageRequest {
    fn from(request: AnalyzeExternalPackageAppRequest) -> Self {
        Self {
            source_path: request.source_path,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateExternalPackageBundleAppRequest {
    pub source_path: PathBuf,
    pub source_flavor: WowFlavor,
    pub source_platform: Option<HostPlatform>,
    pub supported_targets: Vec<WowFlavor>,
    pub output_path: Option<PathBuf>,
    pub package_id: Option<String>,
    pub package_name: Option<String>,
    pub created_by: Option<String>,
    pub description: Option<String>,
    pub apply_defaults: Option<BundleApplyDefaultsValue>,
}

impl From<CreateExternalPackageBundleAppRequest> for DomainCreateExternalPackageBundleRequest {
    fn from(request: CreateExternalPackageBundleAppRequest) -> Self {
        Self {
            source_path: request.source_path,
            source_flavor: request.source_flavor,
            source_platform: request.source_platform,
            supported_targets: request.supported_targets,
            output_path: request.output_path,
            package_id: request.package_id,
            package_name: request.package_name,
            created_by: request.created_by,
            description: request.description,
            apply_defaults: request.apply_defaults.map(Into::into),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanExternalPackageApplyAppRequest {
    pub external_package: CreateExternalPackageBundleAppRequest,
    pub installation: ResolvedInstallationValue,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl From<PlanExternalPackageApplyAppRequest> for DomainPlanExternalPackageApplyRequest {
    fn from(request: PlanExternalPackageApplyAppRequest) -> Self {
        Self {
            external_package: request.external_package.into(),
            installation: request.installation.into(),
            apply_mappings: request.apply_mappings.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApplyExternalPackageAppRequest {
    pub external_package: CreateExternalPackageBundleAppRequest,
    pub installation: ResolvedInstallationValue,
    pub dry_run: bool,
    pub backup_output_path: Option<PathBuf>,
    pub apply_mappings: BundleApplyMappingsValue,
}

impl From<ApplyExternalPackageAppRequest> for DomainApplyExternalPackageRequest {
    fn from(request: ApplyExternalPackageAppRequest) -> Self {
        Self {
            external_package: request.external_package.into(),
            installation: request.installation.into(),
            dry_run: request.dry_run,
            backup_output_path: request.backup_output_path,
            apply_mappings: request.apply_mappings.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InspectInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavor>,
}

#[derive(Debug, Clone)]
pub struct ResolveInstallationRequest {
    pub path: PathBuf,
    pub flavor: Option<WowFlavor>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::app::{
        AddonPackageMetadataValue, BundleApplyDefaultsValue, BundleCharacterMappingOverrideValue,
        BundleCharacterResourceValue, BundleManifestValue, BundleMappingRulesValue,
        BundlePackageValue, BundleResourcesValue, BundleSourceValue, ResourceApplyPolicyValue,
    };
    use crate::core::manifest::{CharacterMappingMode, ResourceApplyPolicy};

    #[test]
    fn apply_bundle_request_converts_app_owned_apply_mappings() {
        let domain: DomainUnpackBundleRequest = ApplyBundleAppRequest {
            bundle_path: PathBuf::from("bundle.zip"),
            installation: sample_installation(),
            dry_run: true,
            backup_output_path: Some(PathBuf::from("backup")),
            apply_mappings: BundleApplyMappingsValue {
                target_account: Some("AccountA".to_string()),
                target_server: Some("Illidan".to_string()),
                target_character: Some("Main".to_string()),
                selected_accounts: vec!["AccountA".to_string()],
                all_accounts: true,
                characters: vec![BundleCharacterMappingOverrideValue {
                    source_account: Some("SourceAccount".to_string()),
                    source_server: "Stormrage".to_string(),
                    source_character: "SourceMain".to_string(),
                    target_account: Some("TargetAccount".to_string()),
                    target_server: "Illidan".to_string(),
                    target_character: "TargetMain".to_string(),
                }],
            },
        }
        .into();

        assert_eq!(domain.bundle_path, PathBuf::from("bundle.zip"));
        assert!(domain.dry_run);
        assert_eq!(
            domain.apply_mappings.target_account.as_deref(),
            Some("AccountA")
        );
        assert_eq!(
            domain.apply_mappings.target_server.as_deref(),
            Some("Illidan")
        );
        assert_eq!(
            domain.apply_mappings.target_character.as_deref(),
            Some("Main")
        );
        assert_eq!(domain.apply_mappings.selected_accounts, vec!["AccountA"]);
        assert!(domain.apply_mappings.all_accounts);
        assert_eq!(domain.apply_mappings.characters.len(), 1);
        assert_eq!(
            domain.apply_mappings.characters[0]
                .source_account
                .as_deref(),
            Some("SourceAccount")
        );
    }

    #[test]
    fn create_external_package_request_converts_app_owned_apply_defaults() {
        let domain: DomainCreateExternalPackageBundleRequest =
            CreateExternalPackageBundleAppRequest {
                source_path: PathBuf::from("author-ui.zip"),
                source_flavor: WowFlavor::Retail,
                source_platform: Some(HostPlatform::Windows),
                supported_targets: vec![WowFlavor::Retail, WowFlavor::Classic],
                output_path: Some(PathBuf::from("out")),
                package_id: Some("author-ui".to_string()),
                package_name: Some("Author UI".to_string()),
                created_by: Some("tester".to_string()),
                description: Some("normalized".to_string()),
                apply_defaults: Some(BundleApplyDefaultsValue {
                    create_backup: false,
                    addons: ResourceApplyPolicyValue::Mirror,
                    wtf_common: ResourceApplyPolicyValue::Share,
                    wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
                    fonts: ResourceApplyPolicyValue::Preserve,
                    interface_assets: ResourceApplyPolicyValue::Sync,
                }),
            }
            .into();

        let apply_defaults = domain.apply_defaults.expect("apply defaults");
        assert!(!apply_defaults.create_backup);
        assert_eq!(apply_defaults.addons, ResourceApplyPolicy::Mirror);
        assert_eq!(apply_defaults.wtf_common, ResourceApplyPolicy::Share);
        assert_eq!(
            apply_defaults.wtf_characters,
            ResourceApplyPolicy::ReplaceSelected
        );
        assert_eq!(apply_defaults.fonts, ResourceApplyPolicy::Preserve);
        assert_eq!(apply_defaults.interface_assets, ResourceApplyPolicy::Sync);
    }

    #[test]
    fn pack_bundle_request_converts_app_owned_manifest() {
        let domain: DomainPackBundleRequest = PackBundleAppRequest {
            installation: sample_installation(),
            manifest: sample_manifest(),
            output_path: Some(PathBuf::from("bundle.zip")),
            manifest_base_dir: Some(PathBuf::from("manifest-dir")),
        }
        .into();

        assert_eq!(domain.manifest.schema_version, 1);
        assert_eq!(domain.manifest.package.id, "author-ui");
        assert_eq!(domain.manifest.source.flavor, WowFlavor::Retail);
        assert_eq!(domain.manifest.resources.addons, vec!["WeakAuras"]);
        assert_eq!(domain.manifest.resources.wtf_characters.len(), 1);
        assert_eq!(
            domain.manifest.mapping.character_mode,
            CharacterMappingMode::Explicit
        );
        assert_eq!(domain.manifest.apply.addons, ResourceApplyPolicy::Mirror);
    }

    #[test]
    fn install_addon_request_converts_app_owned_metadata() {
        let domain: DomainInstallAddonRequest = InstallAddonAppRequest {
            installation: sample_installation(),
            source: "https://example.invalid/weakauras.zip".to_string(),
            dry_run: false,
            backup_output_path: Some(PathBuf::from("backup")),
            replace_existing: true,
            metadata: Some(AddonPackageMetadataValue {
                index_name: Some("curated".to_string()),
                index_package_id: Some("weakauras".to_string()),
                package_name: Some("WeakAuras".to_string()),
                version: Some("1.2.3".to_string()),
                source_url: Some("https://example.invalid/weakauras.zip".to_string()),
                website_url: Some("https://example.invalid/weakauras".to_string()),
                source_sha256: Some("abc123".to_string()),
                supported_flavors: vec!["retail".to_string()],
            }),
        }
        .into();

        let metadata = domain.metadata.expect("metadata");
        assert_eq!(metadata.index_name.as_deref(), Some("curated"));
        assert_eq!(metadata.index_package_id.as_deref(), Some("weakauras"));
        assert_eq!(metadata.package_name.as_deref(), Some("WeakAuras"));
        assert_eq!(metadata.version.as_deref(), Some("1.2.3"));
        assert_eq!(
            metadata.source_url.as_deref(),
            Some("https://example.invalid/weakauras.zip")
        );
        assert_eq!(metadata.supported_flavors, vec!["retail"]);
    }

    fn sample_installation() -> ResolvedInstallationValue {
        ResolvedInstallationValue {
            platform: HostPlatform::Windows,
            flavor: WowFlavor::Retail,
            product_root: PathBuf::from("World of Warcraft"),
            flavor_root: PathBuf::from("World of Warcraft/_retail_"),
            interface_dir: PathBuf::from("World of Warcraft/_retail_/Interface"),
            addon_dir: PathBuf::from("World of Warcraft/_retail_/Interface/AddOns"),
            wtf_dir: PathBuf::from("World of Warcraft/_retail_/WTF"),
            fonts_dir: PathBuf::from("World of Warcraft/_retail_/Fonts"),
        }
    }

    fn sample_manifest() -> BundleManifestValue {
        BundleManifestValue {
            schema_version: 1,
            package: BundlePackageValue {
                id: "author-ui".to_string(),
                name: "Author UI".to_string(),
                created_by: "tester".to_string(),
                description: Some("fixture".to_string()),
            },
            source: BundleSourceValue {
                flavor: WowFlavor::Retail,
                platform: Some(HostPlatform::Windows),
                exported_at: None,
                supported_targets: vec![WowFlavor::Retail],
            },
            resources: BundleResourcesValue {
                addons: vec!["WeakAuras".to_string()],
                wtf_common: true,
                wtf_characters: vec![BundleCharacterResourceValue {
                    source_account: Some("AccountA".to_string()),
                    source_server: "Illidan".to_string(),
                    source_character: "Main".to_string(),
                    target_hint: Some("Main".to_string()),
                }],
                fonts: true,
                interface_assets: vec!["Interface/Buttons".to_string()],
                addon_lock: false,
                addon_indexes: Vec::new(),
            },
            mapping: BundleMappingRulesValue {
                character_mode: CharacterMappingMode::Explicit,
                rewrite_profile_keys: true,
                rewrite_identity_strings: true,
                allow_cross_platform: true,
            },
            apply: BundleApplyDefaultsValue {
                create_backup: true,
                addons: ResourceApplyPolicyValue::Mirror,
                wtf_common: ResourceApplyPolicyValue::Share,
                wtf_characters: ResourceApplyPolicyValue::ReplaceSelected,
                fonts: ResourceApplyPolicyValue::Mirror,
                interface_assets: ResourceApplyPolicyValue::Mirror,
            },
        }
    }
}
