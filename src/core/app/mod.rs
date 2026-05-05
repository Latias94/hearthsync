mod addon;
mod addon_index;
mod addon_lock;
mod addon_policy;
mod backup;
mod bundle;
mod config;
mod extended;
mod external_package;
mod install;
mod live_task;
mod request;
mod response;
mod runtime;
mod runtime_settings;
mod stable;
mod task_support;
mod types;

fn map_owned_vec<TInput, TOutput, FConvert>(values: Vec<TInput>, convert: FConvert) -> Vec<TOutput>
where
    FConvert: FnMut(TInput) -> TOutput,
{
    values.into_iter().map(convert).collect()
}

pub use crate::core::task::{
    CancellationToken, TaskKind, TaskPhase, TaskProgressCode, TaskProgressEvent, TaskProgressSink,
    TaskRun,
};
pub(in crate::core::app) use addon::AddonService;
pub(in crate::core::app) use addon_index::AddonIndexService;
pub(in crate::core::app) use addon_lock::AddonLockService;
pub(in crate::core::app) use addon_policy::AddonPolicyService;
pub(in crate::core::app) use backup::BackupService;
pub(in crate::core::app) use bundle::BundleService;
pub(in crate::core::app) use config::ConfigService;
pub use extended::ExtendedAppServices;
pub(in crate::core::app) use external_package::ExternalPackageService;
pub(in crate::core::app) use install::InstallationService;
pub use live_task::AppLiveTask;
pub(in crate::core::app) use live_task::run_app_live_task;
pub use request::addon::{
    AdoptAddonsAppRequest, InstallAddonAppRequest, ListAddonsRequest, RelinkAddonAppRequest,
    RemoveAddonAppRequest, SearchAddonsRequest, UpdateAddonAppRequest,
};
pub use request::addon_index::{
    AttachAddonIndexAppRequest, InspectAddonIndexRequest, InstallAddonIndexAppRequest,
    RelinkAddonIndexAppRequest, ScaffoldAddonIndexRequest, SuggestAddonIndexRequest,
    UpdateAddonIndexAppRequest,
};
pub use request::addon_lock::{
    AddonLockSourceOverrideRequest, ApplyAddonLockAppRequest, DiffAddonLockRequest,
    InspectAddonLockRequest, PlanAddonLockSyncRequest, VerifyAddonLockRequest,
    WriteAddonLockRequest,
};
pub use request::addon_policy::{
    InspectAddonPolicyRequest, RemoveAddonPolicyAppRequest, SetAddonPolicyAppRequest,
};
pub use request::backup::{CreateBackupAppRequest, ListBackupsRequest, RestoreBackupAppRequest};
pub use request::bundle::{
    ApplyBundleAddonLockAppRequest, ApplyBundleAppRequest, InspectBundleRequest,
    PackBundleAppRequest, PlanBundleAddonLockRequest, PlanBundleApplyRequest,
};
pub use request::config::{
    ApplyConfigAppRequest, ConfigPackageAppRequest, InspectConfigAppRequest,
    PlanConfigApplyAppRequest,
};
pub use request::external_package::{
    AnalyzeExternalPackageAppRequest, ApplyExternalPackageAppRequest,
    CreateExternalPackageBundleAppRequest, PlanExternalPackageApplyAppRequest,
};
pub use request::installation::{InspectInstallationRequest, ResolveInstallationRequest};
pub use request::runtime_settings::SetRuntimeSettingsAppRequest;
pub use response::addon::{
    AddonCachePurgeResult, AddonCacheRepairResult, AddonInventoryResult, AddonSearchCatalogResult,
    AddonSearchProviderFailureResult, AddonSearchResult, AddonSourceKindResult, AddonSourceResult,
    AdoptedAddonPackageResult, InstalledAddonPackageResult, RelinkedAddonPackageResult,
    RemovedAddonPackageResult, TrackedAddonPackageResult, TrackedAddonResult,
    UpdatedAddonPackageResult,
};
pub use response::addon_index::{
    AddonIndexAttachPackageResult, AddonIndexAttachPackageStatusResult, AddonIndexAttachResult,
    AddonIndexIdentityHintCoverageResult, AddonIndexInspectionResult,
    AddonIndexInspectionWarningCodeResult, AddonIndexInspectionWarningResult,
    AddonIndexInspectionWarningSeverityResult, AddonIndexInstallResult, AddonIndexPackageResult,
    AddonIndexPackageSuggestionResult, AddonIndexPackageSuggestionStatusResult,
    AddonIndexRelinkResult, AddonIndexScaffoldResult, AddonIndexSuggestionResult,
    AddonIndexTrackedMatchStrategyResult, AddonIndexUpdateResult, AddonIndexValidationResult,
};
pub use response::addon_lock::{
    AddonLockApplyResult, AddonLockDiffResult, AddonLockFieldChangeResult,
    AddonLockInspectionResult, AddonLockPackageDiffResult, AddonLockPackageDirectoryIssueResult,
    AddonLockPackageResult, AddonLockPackageSnapshotResult, AddonLockPlanResult,
    AddonLockSyncActionKindResult, AddonLockSyncActionResult, AddonLockVerifyResult,
    AddonLockWriteResult,
};
pub use response::addon_policy::{
    AddonPolicyInspectionResult, AddonPolicyMutationResult, AddonPolicyPackageResult,
};
pub use response::backup::{
    BackupCatalogResult, BackupEntryResult, BackupMetadataResult, CreatedBackupResult,
    RestoredBackupResult,
};
pub use response::bundle::{
    ApplyGroupPoliciesResult, ApplyOperationResult, ApplyPlanSummaryResult,
    BundleAddonLockApplyResult, BundleAddonLockPlanResult, BundleApplyPlanResult,
    BundleApplyResult, BundleCharacterResourceResult, BundleEntryCountsResult,
    BundleInspectionResult, BundleManifestResult, BundleMappingRulesResult, BundlePackageResult,
    BundleResourcesResult, BundleSourceResult, CharacterMappingResult, CreatedBundleResult,
    GroupPolicyResult, LocalWowAccountResult, LocalWowCharacterResult,
};
pub use response::config::{
    ConfigApplyPlanResult, ConfigApplyResult, ConfigInspectionResult, ConfigPackageEntryResult,
    ConfigPackageSourceKindResult, ConfigPackageSummaryResult, ConfigPublicSharingReasonCodeValue,
    ConfigPublicSharingReasonResult, ConfigPublicSharingSeverityValue,
    ConfigPublicSharingStatusValue, ConfigPublicSharingSummaryResult, ConfigSourceCharacterResult,
    ConfigSourceIdentityResult, ConfigWarningCategoryValue, ConfigWarningCodeValue,
    ConfigWarningGroupResult, ConfigWarningResult, ConfigWtfScopeSummaryResult,
};
pub use response::external_package::{
    ExternalPackageAnalysisResult, ExternalPackageApplyPlanResult, ExternalPackageApplyResult,
    ExternalPackageBundleHandle, ExternalPackageBundleResult, ExternalPackageEntryResult,
    ExternalPackagePublicSharingReasonCodeValue, ExternalPackagePublicSharingReasonResult,
    ExternalPackagePublicSharingSeverityValue, ExternalPackagePublicSharingStatusValue,
    ExternalPackagePublicSharingSummaryResult, ExternalPackageSourceCharacterResult,
    ExternalPackageSourceIdentityResult, ExternalPackageSourceKindResult,
    ExternalPackageSummaryResult, ExternalPackageWarningGroupResult, ExternalPackageWarningResult,
    ExternalPackageWtfScopeSummaryResult,
};
pub use response::installation::{
    InstallationHealthResult, InstallationInspectionResult, InstallationScanResult,
};
pub use response::runtime_settings::{
    RuntimeSettingsInspectionResult, RuntimeSettingsMutationResult,
};
pub use runtime::{AppRuntime, AppRuntimeBuilder};
pub(in crate::core::app) use runtime_settings::RuntimeSettingsService;
pub(crate) use runtime_settings::load_persisted_runtime_settings_value;
#[cfg(test)]
pub(crate) use runtime_settings::runtime_settings_path_guard;
pub use stable::StableAppServices;
pub use types::addon::{
    AddonDependencyResolutionCapabilityValue, AddonDependencyResolutionStrategyValue,
    AddonManagementCapabilitiesValue, AddonPackageMetadataValue, AddonPolicyPinValue,
    AddonProviderModeValue, AddonProviderOptionsValue, AddonProviderRetryPolicyValue,
    AddonProviderSourceCapabilityValue, AddonReleaseChannelValue, AddonSourceFamilyValue,
    AddonStatePathsValue, AddonStateStorageValue, AppRuntimeCapabilitiesValue,
    HttpNoValidatorCachePolicyValue,
};
pub use types::backup::BackupGroupValue;
pub use types::bundle::{
    ApplyActionValue, ApplyGroupValue, BundleApplyDefaultsValue, BundleApplyMappingsValue,
    BundleCharacterMappingOverrideValue, BundleCharacterResourceValue, BundleManifestValue,
    BundleMappingRulesValue, BundlePackageValue, BundleResourcesValue, BundleSourceValue,
    CharacterMappingModeValue, ResourceApplyPolicyValue, WtfScopeRiskValue, WtfScopeValue,
};
pub use types::external_package::{
    ExternalPackageLayoutValue, ExternalPackageSharingModeValue,
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
};
pub use types::install::{
    HealthStatusValue, HostPlatformValue, ResolvedInstallationValue, WowFlavorValue,
};
pub use types::runtime::{
    AppRuntimeDiagnosticsValue, ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue,
    ExternalHelperPolicyValue, HelperStrategyValue, RuntimeSettingsValue,
};
