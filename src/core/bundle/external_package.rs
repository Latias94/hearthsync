mod analysis;
mod analyze;
mod classify;
mod create_bundle;
mod manifest;
mod materialize;
mod normalized;
mod plan;
mod prepare;
mod projection;
mod source;
mod source_entry;
mod tasks;
mod types;

pub use analyze::analyze_external_package;
pub use create_bundle::create_external_package_bundle;
pub(crate) use manifest::author_package_apply_defaults;
pub use plan::plan_external_package_apply;
use source_entry::SourceEntry;
pub use tasks::{
    analyze_external_package_task, apply_external_package, apply_external_package_task,
    plan_external_package_apply_task,
};
pub use types::{
    AnalyzeExternalPackageRequest, AppliedExternalPackage, ApplyExternalPackageRequest,
    CreateExternalPackageBundleRequest, ExternalPackageAnalysis, ExternalPackageApplyPlan,
    ExternalPackageEntry, ExternalPackageSourceKind, ExternalPackageSummary,
    ExternalPackageWarning, ExternalPackageWarningCategory, ExternalPackageWarningCode,
    ExternalPackageWarningGroup, PlanExternalPackageApplyRequest, PreparedExternalPackageBundle,
};
