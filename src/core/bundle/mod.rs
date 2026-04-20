mod addon_lock;
mod addon_source_archive;
mod apply;
mod apply_model;
mod apply_policy;
mod apply_source;
mod archive_read;
mod character_mapping;
mod character_mapping_match;
mod constants;
mod entry_layout;
mod entry_plan;
mod execution;
mod external_package;
mod packing;
mod planner;
mod shared;
mod target_accounts;
#[cfg(test)]
mod tests;
mod types;
mod wtf_archive;
mod wtf_scope;
mod zip_write;

pub use addon_lock::{apply_bundle_addon_lock, plan_bundle_addon_lock};
pub use apply::{unpack_bundle, unpack_bundle_task};
pub(crate) use external_package::author_package_apply_defaults;
pub use external_package::{
    AnalyzeExternalPackageRequest, AppliedExternalPackage, ApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest, ExternalPackageAnalysis, ExternalPackageApplyPlan,
    ExternalPackageEntry, ExternalPackageSourceKind, ExternalPackageSummary,
    ExternalPackageWarning, ExternalPackageWarningCategory, ExternalPackageWarningCode,
    ExternalPackageWarningGroup, PlanExternalPackageApplyRequest, PreparedExternalPackageBundle,
    analyze_external_package, analyze_external_package_task, apply_external_package,
    apply_external_package_task, create_external_package_bundle, plan_external_package_apply,
    plan_external_package_apply_task,
};
pub use packing::{inspect_bundle, load_apply_mappings, pack_bundle};
pub use planner::pipeline::plan_bundle_apply;
pub use types::{
    ApplyAction, ApplyGroup, ApplyGroupPolicies, ApplyOperation, ApplyPlanSummary,
    BundleAddonLockApply, BundleAddonLockApplyRequest, BundleAddonLockPlan, BundleApplyMappings,
    BundleApplyPlan, BundleEntryCounts, BundleInspection, CharacterMappingOverride, CreatedBundle,
    GroupPolicy, PackBundleRequest, UnpackBundleRequest, UnpackedBundle, WtfScope,
};
