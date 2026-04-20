use crate::core::task::TaskKind;

use super::super::apply_model::prepared::PreparedApplyOperation;
use super::super::types::ApplyAction;

#[derive(Debug, Clone, Copy)]
pub(in crate::core::bundle) enum BundleApplyTaskContext {
    BundleApply,
    ExternalPackageApply,
}

impl BundleApplyTaskContext {
    pub(super) fn task_kind(self) -> TaskKind {
        match self {
            Self::BundleApply => TaskKind::BundleApply,
            Self::ExternalPackageApply => TaskKind::ExternalPackageApply,
        }
    }

    pub(super) fn planning_message(self, operation_count: usize) -> String {
        match self {
            Self::BundleApply => {
                format!("Prepared bundle apply plan with {operation_count} operation(s)")
            }
            Self::ExternalPackageApply => {
                format!("Prepared external package apply plan with {operation_count} operation(s)")
            }
        }
    }

    pub(super) fn dry_run_completed_message(self) -> &'static str {
        match self {
            Self::BundleApply => "Bundle dry run completed without filesystem writes",
            Self::ExternalPackageApply => {
                "External package dry run completed without filesystem writes"
            }
        }
    }

    pub(super) fn backup_message(self) -> &'static str {
        match self {
            Self::BundleApply => "Creating backup checkpoint before bundle apply",
            Self::ExternalPackageApply => {
                "Creating backup checkpoint before external package apply"
            }
        }
    }

    pub(super) fn backup_label(self) -> &'static str {
        match self {
            Self::BundleApply => "bundle-apply",
            Self::ExternalPackageApply => "external-package-apply",
        }
    }

    pub(super) fn failure_label(self) -> &'static str {
        match self {
            Self::BundleApply => "bundle apply",
            Self::ExternalPackageApply => "external-package apply",
        }
    }

    pub(super) fn executing_message(self, operation_count: usize) -> String {
        match self {
            Self::BundleApply => {
                format!("Executing {operation_count} planned bundle operation(s)")
            }
            Self::ExternalPackageApply => {
                format!("Executing {operation_count} planned external package operation(s)")
            }
        }
    }

    pub(super) fn operation_message(
        self,
        operation_index: usize,
        operation_count: usize,
        operation: &PreparedApplyOperation,
    ) -> String {
        let action = match operation.action {
            ApplyAction::Remove => "remove",
            ApplyAction::Add => "add",
            ApplyAction::Replace => "replace",
            ApplyAction::Skip => "skip",
            ApplyAction::Preserve => "preserve",
        };
        let target = match self {
            Self::BundleApply => "bundle",
            Self::ExternalPackageApply => "external package",
        };

        format!(
            "Executing {target} operation {}/{}: {action} `{}`",
            operation_index + 1,
            operation_count,
            operation.destination.display()
        )
    }

    pub(super) fn completed_message(self, written_files: usize) -> String {
        match self {
            Self::BundleApply => {
                format!("Bundle apply completed with {written_files} written file(s)")
            }
            Self::ExternalPackageApply => {
                format!("External package apply completed with {written_files} written file(s)")
            }
        }
    }
}
