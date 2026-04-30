use super::*;

pub(super) fn format_tracked_addon_names(addons: &[TrackedAddonResult]) -> String {
    addons
        .iter()
        .map(|addon| addon.directory_name.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn format_tracked_package_summaries(packages: &[TrackedAddonPackageResult]) -> String {
    if packages.is_empty() {
        "none".to_string()
    } else {
        packages
            .iter()
            .map(|package| {
                format!(
                    "{} [{}]",
                    package.package_id,
                    format_tracked_addon_names(&package.addons)
                )
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

pub(super) fn format_addon_index_warning_code(
    code: &AddonIndexInspectionWarningCodeResult,
) -> &'static str {
    match code {
        AddonIndexInspectionWarningCodeResult::MissingMatchPackageIds => {
            "missing_match_package_ids"
        }
        AddonIndexInspectionWarningCodeResult::MissingAddonDirectories => {
            "missing_addon_directories"
        }
        AddonIndexInspectionWarningCodeResult::MissingExactIdentityHints => {
            "missing_exact_identity_hints"
        }
    }
}

pub(super) fn format_addon_index_warning_severity(
    severity: &AddonIndexInspectionWarningSeverityResult,
) -> &'static str {
    match severity {
        AddonIndexInspectionWarningSeverityResult::Blocking => "blocking",
        AddonIndexInspectionWarningSeverityResult::Advisory => "advisory",
    }
}

pub(super) fn format_addon_index_match_strategy(
    strategy: &AddonIndexTrackedMatchStrategyResult,
) -> &'static str {
    match strategy {
        AddonIndexTrackedMatchStrategyResult::StoredIndexPackageId => "stored_index_package_id",
        AddonIndexTrackedMatchStrategyResult::ExactPackageId => "exact_package_id",
        AddonIndexTrackedMatchStrategyResult::CuratedMatchPackageId => "curated_match_package_id",
        AddonIndexTrackedMatchStrategyResult::SourceIdentity => "source_identity",
        AddonIndexTrackedMatchStrategyResult::SourceFamilyIdentity => "source_family_identity",
        AddonIndexTrackedMatchStrategyResult::DisplayName => "display_name",
        AddonIndexTrackedMatchStrategyResult::AddonDirectories => "addon_directories",
        AddonIndexTrackedMatchStrategyResult::AddonDirectoryOverlap => "addon_directory_overlap",
    }
}

pub(super) fn format_addon_index_suggestion_status(
    status: &AddonIndexPackageSuggestionStatusResult,
) -> &'static str {
    match status {
        AddonIndexPackageSuggestionStatusResult::Suggested => "suggested",
        AddonIndexPackageSuggestionStatusResult::Complete => "complete",
        AddonIndexPackageSuggestionStatusResult::NoLocalMatch => "no_local_match",
        AddonIndexPackageSuggestionStatusResult::AmbiguousLocalMatch => "ambiguous_local_match",
    }
}

pub(super) fn format_addon_index_attach_status(
    status: &AddonIndexAttachPackageStatusResult,
) -> &'static str {
    match status {
        AddonIndexAttachPackageStatusResult::WouldAttach => "would_attach",
        AddonIndexAttachPackageStatusResult::Attached => "attached",
        AddonIndexAttachPackageStatusResult::AlreadyAttached => "already_attached",
        AddonIndexAttachPackageStatusResult::NoLocalMatch => "no_local_match",
        AddonIndexAttachPackageStatusResult::AmbiguousLocalMatch => "ambiguous_local_match",
        AddonIndexAttachPackageStatusResult::AddonDirectoryMismatch => "addon_directory_mismatch",
        AddonIndexAttachPackageStatusResult::PrepareFailed => "prepare_failed",
        AddonIndexAttachPackageStatusResult::SkippedUnsupportedFlavor => {
            "skipped_unsupported_flavor"
        }
    }
}
