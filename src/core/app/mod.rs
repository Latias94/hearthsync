mod addon;
mod addon_index;
mod addon_lock;
mod backup;
mod bundle;
mod config;
mod extended;
mod external_package;
mod install;
mod request;
mod response;
mod runtime;
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
pub(in crate::core::app) use backup::BackupService;
pub(in crate::core::app) use bundle::BundleService;
pub(in crate::core::app) use config::ConfigService;
pub use extended::ExtendedAppServices;
pub(in crate::core::app) use external_package::ExternalPackageService;
pub(in crate::core::app) use install::InstallationService;
pub use request::addon::{
    InstallAddonAppRequest, ListAddonsRequest, RemoveAddonAppRequest, SearchAddonsRequest,
    UpdateAddonAppRequest,
};
pub use request::addon_index::{
    InspectAddonIndexRequest, InstallAddonIndexAppRequest, UpdateAddonIndexAppRequest,
};
pub use request::addon_lock::{
    AddonLockSourceOverrideRequest, ApplyAddonLockAppRequest, DiffAddonLockRequest,
    InspectAddonLockRequest, PlanAddonLockSyncRequest, VerifyAddonLockRequest,
    WriteAddonLockRequest,
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
pub use response::addon::{
    AddonInventoryResult, AddonSearchCatalogResult, AddonSearchResult, AddonSourceKindResult,
    AddonSourceResult, InstalledAddonPackageResult, RemovedAddonPackageResult,
    TrackedAddonPackageResult, TrackedAddonResult, UpdatedAddonPackageResult,
};
pub use response::addon_index::{
    AddonIndexInspectionResult, AddonIndexInstallResult, AddonIndexPackageResult,
    AddonIndexUpdateResult,
};
pub use response::addon_lock::{
    AddonLockApplyResult, AddonLockDiffResult, AddonLockFieldChangeResult,
    AddonLockInspectionResult, AddonLockPackageDiffResult, AddonLockPackageDirectoryIssueResult,
    AddonLockPackageResult, AddonLockPackageSnapshotResult, AddonLockPlanResult,
    AddonLockSyncActionResult, AddonLockVerifyResult, AddonLockWriteResult,
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
pub use response::config::{ConfigApplyPlanResult, ConfigApplyResult, ConfigInspectionResult};
pub use response::external_package::{
    ExternalPackageAnalysisResult, ExternalPackageApplyPlanResult, ExternalPackageApplyResult,
    ExternalPackageBundleHandle, ExternalPackageBundleResult, ExternalPackageEntryResult,
    ExternalPackageSummaryResult, ExternalPackageWarningGroupResult, ExternalPackageWarningResult,
};
pub use response::installation::{
    InstallationHealthResult, InstallationInspectionResult, InstallationScanResult,
};
pub use runtime::AppRuntime;
pub use stable::StableAppServices;
pub use types::addon::{
    AddonPackageMetadataValue, AddonProviderModeValue, AddonProviderOptionsValue,
    AddonProviderRetryPolicyValue, AppRuntimeCapabilitiesValue,
};
pub use types::backup::BackupGroupValue;
pub use types::bundle::{
    ApplyActionValue, ApplyGroupValue, BundleApplyDefaultsValue, BundleApplyMappingsValue,
    BundleCharacterMappingOverrideValue, BundleCharacterResourceValue, BundleManifestValue,
    BundleMappingRulesValue, BundlePackageValue, BundleResourcesValue, BundleSourceValue,
    CharacterMappingModeValue, ResourceApplyPolicyValue, WtfScopeValue,
};
pub use types::external_package::{
    ExternalPackageWarningCategoryValue, ExternalPackageWarningCodeValue,
};
pub use types::install::{
    HealthStatusValue, HostPlatformValue, ResolvedInstallationValue, WowFlavorValue,
};
pub use types::runtime::{
    ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue, ExternalHelperPolicyValue,
    HelperStrategyValue,
};
