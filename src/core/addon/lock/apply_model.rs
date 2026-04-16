use super::*;

#[derive(Debug)]
pub(super) struct PreparedAddonLockApply {
    pub(super) remove_packages: Vec<TrackedAddonPackage>,
    pub(super) update_current_packages: Vec<TrackedAddonPackage>,
    pub(super) update_prepared_packages: Vec<PreparedAddonPackage>,
    pub(super) install_prepared_packages: Vec<PreparedAddonPackage>,
    pub(super) metadata_actions: Vec<MetadataOnlyAddonLockAction>,
}

impl PreparedAddonLockApply {
    pub(super) fn is_empty(&self) -> bool {
        self.remove_packages.is_empty()
            && self.update_current_packages.is_empty()
            && self.install_prepared_packages.is_empty()
            && self.metadata_actions.is_empty()
    }
}

#[derive(Debug, Clone)]
pub(super) struct MetadataOnlyAddonLockAction {
    pub(super) current: TrackedAddonPackage,
    pub(super) expected: AddonLockPackage,
}

pub(super) fn metadata_from_lock_package(
    package: &AddonLockPackage,
) -> Option<AddonPackageMetadata> {
    let metadata = AddonPackageMetadata {
        index_name: package.index_name.clone(),
        index_package_id: package.index_package_id.clone(),
        package_name: package.name.clone(),
        version: package.version.clone(),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.source_sha256.clone(),
        supported_flavors: Vec::new(),
    };
    (metadata != AddonPackageMetadata::default()).then_some(metadata)
}
