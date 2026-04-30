use super::*;

pub(super) fn metadata_from_index_package(
    index: &AddonIndex,
    package: &AddonIndexPackage,
) -> AddonPackageMetadata {
    AddonPackageMetadata {
        index_name: Some(index.name.clone()),
        index_package_id: Some(package.id.clone()),
        package_name: Some(package.name.clone()),
        version: Some(package.version.clone()),
        source_url: package.source_url.clone(),
        website_url: package.website_url.clone(),
        source_sha256: package.sha256.clone(),
        supported_flavors: package.supported_flavors.clone(),
    }
}

pub(super) fn resolved_index_package_for_matching(
    index_path: &std::path::Path,
    package: &AddonIndexPackage,
) -> AddonIndexPackage {
    let mut resolved = package.clone();
    if let Ok(source) = resolve_index_package_source(index_path, &package.source) {
        resolved.source = source;
    }
    resolved
}

pub(super) fn remap_cancelled_task_kind(
    error: AppError,
    from_task: TaskKind,
    to_task: TaskKind,
) -> AppError {
    match error {
        AppError::Cancelled(message) => {
            AppError::Cancelled(message.replace(from_task.as_str(), to_task.as_str()))
        }
        other => other,
    }
}

pub(super) struct RemappedTaskProgressSink<'a, TProgress> {
    pub(super) inner: &'a mut TProgress,
    pub(super) task: TaskKind,
}

impl<TProgress> TaskProgressSink for RemappedTaskProgressSink<'_, TProgress>
where
    TProgress: TaskProgressSink,
{
    fn push(&mut self, mut event: TaskProgressEvent) {
        event.task = self.task;
        self.inner.push(event);
    }

    fn task_id(&self) -> Option<&str> {
        self.inner.task_id()
    }
}
