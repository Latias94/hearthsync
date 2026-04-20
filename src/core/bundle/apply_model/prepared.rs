use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::core::lua_patch::CharacterMapping;

use super::super::{ApplyAction, BundleApplyPlan, ExternalPackageSourceKind};
use super::preview::PreviewOperation;

#[derive(Debug, Clone)]
pub(in crate::core::bundle) struct PreparedApplyOperation {
    pub(in crate::core::bundle) action: ApplyAction,
    pub(in crate::core::bundle) archive_name: String,
    pub(in crate::core::bundle) destination: PathBuf,
    pub(in crate::core::bundle) rewrites: Vec<CharacterMapping>,
}

#[derive(Debug, Clone)]
pub(in crate::core::bundle) enum PreparedApplySource {
    BundleArchive {
        bundle_path: PathBuf,
    },
    ExternalPackage {
        source_path: PathBuf,
        source_kind: ExternalPackageSourceKind,
        entry_source_map: BTreeMap<String, String>,
    },
}

impl PreparedApplyOperation {
    pub(in crate::core::bundle) fn from_preview(preview_operation: PreviewOperation) -> Self {
        let (
            super::super::ApplyOperation {
                action,
                archive_name,
                destination,
                ..
            },
            rewrites,
        ) = preview_operation.into_parts();

        Self {
            action,
            archive_name,
            destination,
            rewrites,
        }
    }
}

#[derive(Debug)]
pub(in crate::core::bundle) struct PreparedBundleApply {
    pub(in crate::core::bundle) source: PreparedApplySource,
    pub(in crate::core::bundle) plan: BundleApplyPlan,
    pub(in crate::core::bundle) execution_operations: Vec<PreparedApplyOperation>,
}
