use std::path::PathBuf;

use crate::core::app::{
    AddonDependencyResolutionCapabilityValue, AddonIndexPackageResult, AddonLockApplyResult,
    AddonLockDiffResult, AddonLockPackageSnapshotResult, AddonLockPlanResult,
    AddonLockVerifyResult, AddonSourceKindResult, AddonSourceResult, ApplyGroupPoliciesResult,
    BackupGroupValue, BackupMetadataResult, BundleApplyDefaultsValue, BundleCharacterResourceValue,
    BundleManifestValue, BundleMappingRulesValue, BundlePackageValue, BundleResourcesResult,
    BundleResourcesValue, BundleSourceValue, CharacterMappingModeValue, CharacterMappingResult,
    ExternalPackageAnalysisResult, ExternalPackageEntryResult, ExternalPackageLayoutValue,
    ExternalPackagePublicSharingReasonCodeValue, ExternalPackagePublicSharingReasonResult,
    ExternalPackagePublicSharingSeverityValue, ExternalPackagePublicSharingStatusValue,
    ExternalPackagePublicSharingSummaryResult, ExternalPackageSourceCharacterResult,
    ExternalPackageSourceIdentityResult, ExternalPackageSourceKindResult,
    ExternalPackageSummaryResult, ExternalPackageWarningCategoryValue,
    ExternalPackageWarningCodeValue, ExternalPackageWarningGroupResult,
    ExternalPackageWarningResult, ExternalPackageWtfScopeSummaryResult, GroupPolicyResult,
    HostPlatformValue, LocalWowAccountResult, LocalWowCharacterResult, ResourceApplyPolicyValue,
    TrackedAddonPackageResult, TrackedAddonResult, WowFlavorValue, WtfScopeRiskValue,
    WtfScopeValue,
};

pub(super) use crate::cli::test_support::sample_installation;

pub(super) fn sample_snapshot(
    package_id: &str,
    name: Option<&str>,
) -> AddonLockPackageSnapshotResult {
    AddonLockPackageSnapshotResult {
        comparison_key: package_id.to_string(),
        package_id: package_id.to_string(),
        index_name: None,
        index_package_id: None,
        name: name.map(ToString::to_string),
        version: Some("1.0.0".to_string()),
        source: sample_source(),
        source_label: "local.zip".to_string(),
        source_url: None,
        website_url: None,
        source_sha256: None,
        content_sha256: Some("sha256".to_string()),
        addon_directories: vec!["AddonDir".to_string()],
    }
}

pub(super) fn sample_addon_lock_plan() -> AddonLockPlanResult {
    AddonLockPlanResult {
        lock_path: PathBuf::from("addon.lock"),
        installation_root: PathBuf::from("World of Warcraft/_retail_"),
        install_count: 1,
        update_count: 2,
        remove_count: 3,
        metadata_only_count: 4,
        unchanged_count: 5,
        blocked_count: 0,
        untracked_addon_count: 1,
        untracked_addons: vec!["LooseAddon".to_string()],
        action_count: 0,
        actions: Vec::new(),
    }
}

pub(super) fn sample_addon_lock_apply() -> AddonLockApplyResult {
    AddonLockApplyResult {
        lock_path: PathBuf::from("addon.lock"),
        installation_root: PathBuf::from("World of Warcraft/_retail_"),
        install_count: 1,
        update_count: 2,
        remove_count: 3,
        metadata_only_count: 4,
        unchanged_count: 5,
        blocked_count: 0,
        untracked_addon_count: 0,
        untracked_addons: Vec::new(),
        action_count: 0,
        actions: Vec::new(),
        verification: AddonLockVerifyResult {
            lock_path: PathBuf::from("addon.lock"),
            installation_root: PathBuf::from("World of Warcraft/_retail_"),
            tracked_package_count: 0,
            untracked_addon_count: 0,
            untracked_addons: Vec::new(),
            missing_package_count: 0,
            missing_addon_directories: Vec::new(),
            diff: AddonLockDiffResult {
                left_label: "lock".to_string(),
                right_label: "install".to_string(),
                left_package_count: 0,
                right_package_count: 0,
                identical: true,
                unchanged_packages: 0,
                added_package_count: 0,
                removed_package_count: 0,
                changed_package_count: 0,
                added_packages: Vec::new(),
                removed_packages: Vec::new(),
                changed_packages: Vec::new(),
            },
            matches: true,
        },
    }
}

pub(super) fn sample_index_package(package_id: &str, version: &str) -> AddonIndexPackageResult {
    AddonIndexPackageResult {
        id: package_id.to_string(),
        name: package_id.to_string(),
        version: version.to_string(),
        match_package_ids: Vec::new(),
        source: sample_source(),
        source_label: "local.zip".to_string(),
        source_url: None,
        website_url: None,
        sha256: None,
        addon_directories: vec![package_id.to_string()],
        supported_flavors: vec!["retail".to_string()],
    }
}

pub(super) fn sample_tracked_addon(directory_name: &str) -> TrackedAddonResult {
    TrackedAddonResult {
        directory_name: directory_name.to_string(),
        toc_file: Some(format!("{directory_name}.toc")),
        title: Some(directory_name.to_string()),
        version: Some("1.0.0".to_string()),
    }
}

pub(super) fn sample_tracked_package(package_id: &str) -> TrackedAddonPackageResult {
    TrackedAddonPackageResult {
        package_id: package_id.to_string(),
        source: sample_source(),
        source_label: "local.zip".to_string(),
        installed_at: "2026-04-19T12:00:00Z".to_string(),
        updated_at: "2026-04-19T12:00:00Z".to_string(),
        addon_count: 2,
        addons: vec![
            sample_tracked_addon("WeakAuras"),
            sample_tracked_addon("WeakAurasOptions"),
        ],
        metadata: None,
    }
}

pub(super) fn sample_bundle_manifest() -> BundleManifestValue {
    BundleManifestValue {
        schema_version: 1,
        package: BundlePackageValue {
            id: "author-ui".to_string(),
            name: "Author UI".to_string(),
            created_by: "tester".to_string(),
            description: None,
        },
        source: BundleSourceValue {
            flavor: WowFlavorValue::Retail,
            platform: Some(HostPlatformValue::Windows),
            exported_at: Some("2026-04-19T12:00:00Z".to_string()),
            supported_targets: vec![WowFlavorValue::Retail, WowFlavorValue::Classic],
        },
        resources: BundleResourcesValue {
            addons: vec!["WeakAuras".to_string()],
            wtf_common: true,
            wtf_characters: vec![sample_bundle_character_resource()],
            fonts: true,
            interface_assets: vec!["Interface/SharedXML".to_string()],
            addon_lock: true,
            addon_indexes: vec!["addons.toml".to_string()],
        },
        mapping: BundleMappingRulesValue {
            character_mode: CharacterMappingModeValue::Explicit,
            rewrite_profile_keys: true,
            rewrite_identity_strings: true,
            allow_cross_platform: true,
        },
        apply: BundleApplyDefaultsValue::author_package_defaults(),
    }
}

pub(super) fn sample_bundle_character_resource() -> BundleCharacterResourceValue {
    BundleCharacterResourceValue {
        source_account: Some("AccountA".to_string()),
        source_server: "Aegwynn".to_string(),
        source_character: "Hero".to_string(),
        target_hint: None,
    }
}

pub(super) fn sample_local_account(account_name: &str) -> LocalWowAccountResult {
    LocalWowAccountResult {
        account_name: account_name.to_string(),
        account_dir: PathBuf::from(format!("WTF/Account/{account_name}")),
        saved_variables_dir: PathBuf::from(format!("WTF/Account/{account_name}/SavedVariables")),
        characters: vec![LocalWowCharacterResult {
            server: "Illidan".to_string(),
            character: "Main".to_string(),
            character_dir: PathBuf::from(format!("WTF/Account/{account_name}/Illidan/Main")),
        }],
    }
}

pub(super) fn sample_character_mapping() -> CharacterMappingResult {
    CharacterMappingResult {
        source_account: Some("AccountA".to_string()),
        source_server: "Aegwynn".to_string(),
        source_character: "Hero".to_string(),
        target_account: "TargetAccount".to_string(),
        target_server: "Illidan".to_string(),
        target_character: "Main".to_string(),
    }
}

pub(super) fn sample_group_policies() -> ApplyGroupPoliciesResult {
    ApplyGroupPoliciesResult {
        addons: GroupPolicyResult {
            policy: ResourceApplyPolicyValue::Mirror,
        },
        wtf_common: GroupPolicyResult {
            policy: ResourceApplyPolicyValue::Share,
        },
        wtf_characters: GroupPolicyResult {
            policy: ResourceApplyPolicyValue::ReplaceSelected,
        },
        fonts: GroupPolicyResult {
            policy: ResourceApplyPolicyValue::Preserve,
        },
        interface_assets: GroupPolicyResult {
            policy: ResourceApplyPolicyValue::Mirror,
        },
        metadata: GroupPolicyResult {
            policy: ResourceApplyPolicyValue::Preserve,
        },
    }
}

pub(super) fn sample_external_package_analysis() -> ExternalPackageAnalysisResult {
    ExternalPackageAnalysisResult {
        source_path: PathBuf::from("C:\\temp\\author-ui.zip"),
        source_kind: ExternalPackageSourceKindResult::ZipArchive,
        layout: ExternalPackageLayoutValue::Generic,
        package_id: "author-ui".to_string(),
        package_name: "Author UI".to_string(),
        entry_count: 0,
        entries: Vec::<ExternalPackageEntryResult>::new(),
        resources: BundleResourcesResult {
            addons: vec!["WeakAuras".to_string()],
            addon_count: 1,
            wtf_common: true,
            wtf_character_count: 1,
            wtf_characters: vec![sample_bundle_character_resource()],
            fonts: true,
            interface_assets: vec!["Interface/SharedXML".to_string()],
            interface_asset_count: 1,
            addon_lock: false,
            addon_indexes: Vec::new(),
        },
        summary: ExternalPackageSummaryResult {
            total_files: 12,
            normalized_files: 10,
            ignored_files: 2,
            addons: 1,
            wtf_common: 1,
            wtf_characters: 1,
            fonts: 1,
            interface_assets: 1,
            warning_count: 1,
            addon_warning_count: 1,
            wtf_warning_count: 0,
            warning_groups: vec![ExternalPackageWarningGroupResult {
                category: ExternalPackageWarningCategoryValue::Addon,
                code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
                count: 1,
            }],
            wtf_scopes: vec![ExternalPackageWtfScopeSummaryResult {
                scope: WtfScopeValue::AccountSavedVariables,
                risk: WtfScopeRiskValue::High,
                count: 1,
            }],
            source_identities: ExternalPackageSourceIdentityResult {
                source_accounts: vec!["AccountA".to_string()],
                source_characters: vec![ExternalPackageSourceCharacterResult {
                    source_account: Some("AccountA".to_string()),
                    source_server: "Aegwynn".to_string(),
                    source_character: "Hero".to_string(),
                }],
                entries_with_source_account: 1,
                entries_with_source_character: 1,
            },
            public_sharing: ExternalPackagePublicSharingSummaryResult {
                status: ExternalPackagePublicSharingStatusValue::ReviewRequired,
                public_ready: false,
                review_required_count: 4,
                advisory_count: 0,
                reasons: vec![
                    ExternalPackagePublicSharingReasonResult {
                        severity: ExternalPackagePublicSharingSeverityValue::ReviewRequired,
                        code: ExternalPackagePublicSharingReasonCodeValue::NormalizationWarnings,
                        count: 1,
                        message: "package normalization produced warnings".to_string(),
                    },
                    ExternalPackagePublicSharingReasonResult {
                        severity: ExternalPackagePublicSharingSeverityValue::ReviewRequired,
                        code: ExternalPackagePublicSharingReasonCodeValue::HighRiskWtfScope,
                        count: 1,
                        message: "package contains high-risk WTF data".to_string(),
                    },
                    ExternalPackagePublicSharingReasonResult {
                        severity: ExternalPackagePublicSharingSeverityValue::ReviewRequired,
                        code: ExternalPackagePublicSharingReasonCodeValue::SourceAccountIdentity,
                        count: 1,
                        message: "package paths expose source account identity".to_string(),
                    },
                    ExternalPackagePublicSharingReasonResult {
                        severity: ExternalPackagePublicSharingSeverityValue::ReviewRequired,
                        code: ExternalPackagePublicSharingReasonCodeValue::SourceCharacterIdentity,
                        count: 1,
                        message: "package paths expose source character identity".to_string(),
                    },
                ],
            },
        },
        warnings: vec![ExternalPackageWarningResult {
            category: ExternalPackageWarningCategoryValue::Addon,
            code: ExternalPackageWarningCodeValue::AddonRootNotDetected,
            source_path: "AuthorUI/README.txt".to_string(),
            message: "ignored addon entry".to_string(),
        }],
    }
}

pub(super) fn sample_backup_metadata() -> BackupMetadataResult {
    BackupMetadataResult {
        schema_version: 1,
        created_at: "2026-04-19T12:00:00Z".to_string(),
        label: Some("before apply".to_string()),
        flavor: "retail".to_string(),
        flavor_root: PathBuf::from("C:\\Games\\World of Warcraft\\_retail_"),
        group_count: 2,
        groups: vec![BackupGroupValue::Addons, BackupGroupValue::Wtf],
    }
}

pub(super) fn sample_source() -> AddonSourceResult {
    AddonSourceResult {
        kind: AddonSourceKindResult::LocalArchive,
        display_name: "local.zip".to_string(),
        dependency_resolution_capability: AddonDependencyResolutionCapabilityValue::Unsupported,
        local_archive_path: Some(PathBuf::from("local.zip")),
        url: None,
        mod_id: None,
        file_id: None,
        owner: None,
        repo: None,
        tag: None,
        asset_name: None,
        project_id: None,
        release_id: None,
    }
}
