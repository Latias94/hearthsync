use super::*;

pub(super) fn index_attach_result(
    mut plan: IndexAttachPlan,
    applied: bool,
) -> AddonIndexAttachResult {
    if applied {
        for change in &plan.changes {
            plan.packages[change.package_result_index] =
                ready_attach_result(ReadyAttachResultRequest {
                    package: change.package.clone(),
                    tracked_package_id: &change.tracked_package.package_id,
                    match_strategy: change.match_strategy.clone(),
                    previous_source: change.tracked_package.source.clone(),
                    source: change.next_source.clone(),
                    source_changed: change.source_changed,
                    metadata_changed: change.metadata_changed,
                    applied: true,
                });
        }
    }

    let change_package_count = plan.changes.len();
    let attached_package_count = if applied { change_package_count } else { 0 };
    let already_attached_package_count = plan
        .packages
        .iter()
        .filter(|package| {
            matches!(
                package.status,
                AddonIndexAttachPackageStatus::AlreadyAttached
            )
        })
        .count();
    let blocked_package_count = attach_blocked_package_count(&plan.packages);
    let partial_apply = applied && blocked_package_count > 0;

    AddonIndexAttachResult {
        index_path: plan.index_path,
        index_name: plan.index_name,
        dry_run: plan.dry_run,
        ready: blocked_package_count == 0,
        applied,
        partial_apply,
        registry_path: plan.registry_path,
        index_package_count: plan.index_package_count,
        considered_package_count: plan.considered_package_count,
        change_package_count,
        attached_package_count,
        already_attached_package_count,
        blocked_package_count,
        skipped_unsupported_flavor_package_count: plan.skipped_unsupported_flavor_package_count,
        packages: plan.packages,
    }
}

pub(super) fn result_ready_for_attach(plan: &IndexAttachPlan) -> bool {
    attach_blocked_package_count(&plan.packages) == 0
}

pub(super) fn attach_blocked_package_count(packages: &[AddonIndexAttachPackageResult]) -> usize {
    packages
        .iter()
        .filter(|package| attach_status_is_blocking(&package.status))
        .count()
}

fn attach_status_is_blocking(status: &AddonIndexAttachPackageStatus) -> bool {
    matches!(
        status,
        AddonIndexAttachPackageStatus::NoLocalMatch
            | AddonIndexAttachPackageStatus::AmbiguousLocalMatch
            | AddonIndexAttachPackageStatus::AddonDirectoryMismatch
            | AddonIndexAttachPackageStatus::PrepareFailed
    )
}

pub(super) struct ReadyAttachResultRequest<'a> {
    pub(super) package: AddonIndexPackage,
    pub(super) tracked_package_id: &'a str,
    pub(super) match_strategy: AddonIndexTrackedMatchStrategy,
    pub(super) previous_source: AddonSourceRef,
    pub(super) source: AddonSourceRef,
    pub(super) source_changed: bool,
    pub(super) metadata_changed: bool,
    pub(super) applied: bool,
}

pub(super) fn ready_attach_result(
    request: ReadyAttachResultRequest<'_>,
) -> AddonIndexAttachPackageResult {
    let ReadyAttachResultRequest {
        package,
        tracked_package_id,
        match_strategy,
        previous_source,
        source,
        source_changed,
        metadata_changed,
        applied,
    } = request;
    let status = if applied {
        AddonIndexAttachPackageStatus::Attached
    } else {
        AddonIndexAttachPackageStatus::WouldAttach
    };

    AddonIndexAttachPackageResult {
        package,
        status,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy.clone()),
        previous_source: Some(previous_source),
        source: Some(source),
        source_changed,
        metadata_changed,
        message: attach_change_message(
            tracked_package_id,
            &match_strategy,
            source_changed,
            metadata_changed,
            applied,
        ),
    }
}

pub(super) fn already_attached_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: &str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    source: AddonSourceRef,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AlreadyAttached,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy.clone()),
        previous_source: Some(source.clone()),
        source: Some(source),
        source_changed: false,
        metadata_changed: false,
        message: format!(
            "matched tracked package `{}` by {}; source and curated metadata already match",
            tracked_package_id,
            match_strategy_label(&match_strategy)
        ),
    }
}

pub(super) fn no_local_match_attach_result(
    package: AddonIndexPackage,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::NoLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message: "no tracked addon package from the current registry matched this index package"
            .to_string(),
    }
}

pub(super) fn ambiguous_local_match_attach_result(
    package: AddonIndexPackage,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AmbiguousLocalMatch,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

pub(super) fn addon_directory_mismatch_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: &str,
    match_strategy: AddonIndexTrackedMatchStrategy,
    previous_source: AddonSourceRef,
    source: AddonSourceRef,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::AddonDirectoryMismatch,
        matched_tracked_package_id: Some(tracked_package_id.to_string()),
        match_strategy: Some(match_strategy),
        previous_source: Some(previous_source),
        source: Some(source),
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

pub(super) fn prepare_failed_attach_result(
    package: AddonIndexPackage,
    tracked_package_id: Option<&str>,
    match_strategy: Option<AddonIndexTrackedMatchStrategy>,
    previous_source: Option<AddonSourceRef>,
    source: Option<AddonSourceRef>,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::PrepareFailed,
        matched_tracked_package_id: tracked_package_id.map(|value| value.to_string()),
        match_strategy,
        previous_source,
        source,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

pub(super) fn skipped_unsupported_flavor_attach_result(
    package: AddonIndexPackage,
    message: String,
) -> AddonIndexAttachPackageResult {
    AddonIndexAttachPackageResult {
        package,
        status: AddonIndexAttachPackageStatus::SkippedUnsupportedFlavor,
        matched_tracked_package_id: None,
        match_strategy: None,
        previous_source: None,
        source: None,
        source_changed: false,
        metadata_changed: false,
        message,
    }
}

fn attach_change_message(
    tracked_package_id: &str,
    strategy: &AddonIndexTrackedMatchStrategy,
    source_changed: bool,
    metadata_changed: bool,
    applied: bool,
) -> String {
    let action = match (source_changed, metadata_changed, applied) {
        (true, true, true) => "attached curated source and metadata",
        (true, true, false) => "would attach curated source and metadata",
        (true, false, true) => "relinked the tracked source to the curated source",
        (true, false, false) => "would relink the tracked source to the curated source",
        (false, true, true) => "attached curated metadata",
        (false, true, false) => "would attach curated metadata",
        (false, false, true) => "left the tracked package unchanged",
        (false, false, false) => "would leave the tracked package unchanged",
    };

    format!(
        "matched tracked package `{}` by {}; {} without reinstalling live AddOns",
        tracked_package_id,
        match_strategy_label(strategy),
        action
    )
}

fn match_strategy_label(strategy: &AddonIndexTrackedMatchStrategy) -> &'static str {
    match strategy {
        AddonIndexTrackedMatchStrategy::StoredIndexPackageId => "stored index package id",
        AddonIndexTrackedMatchStrategy::ExactPackageId => "exact package id",
        AddonIndexTrackedMatchStrategy::CuratedMatchPackageId => "curated match_package_ids hint",
        AddonIndexTrackedMatchStrategy::SourceIdentity => "source identity",
        AddonIndexTrackedMatchStrategy::SourceFamilyIdentity => "source family identity",
        AddonIndexTrackedMatchStrategy::DisplayName => "display name",
        AddonIndexTrackedMatchStrategy::AddonDirectories => "addon directories",
        AddonIndexTrackedMatchStrategy::AddonDirectoryOverlap => "addon directory overlap",
    }
}
