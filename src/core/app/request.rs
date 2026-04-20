use std::path::PathBuf;

use crate::core::app::{AppRuntime, HostPlatformValue};

pub(super) mod addon;
pub(super) mod addon_index;
pub(super) mod addon_lock;
pub(super) mod backup;
pub(super) mod bundle;
pub(super) mod external_package;
pub(super) mod installation;

pub(super) trait RuntimeDefaultableRequest: Sized {
    fn apply_runtime_defaults(self, runtime: &AppRuntime) -> Self;

    fn into_domain_with_runtime_defaults<TDomain, FProject>(
        self,
        runtime: &AppRuntime,
        project: FProject,
    ) -> TDomain
    where
        FProject: FnOnce(Self) -> TDomain,
    {
        project(self.apply_runtime_defaults(runtime))
    }
}

pub(super) fn apply_backup_output_default(runtime: &AppRuntime, output: &mut Option<PathBuf>) {
    *output = runtime.backup_output_or_default(output.take());
}

pub(super) fn apply_bundle_output_default(runtime: &AppRuntime, output: &mut Option<PathBuf>) {
    *output = runtime.bundle_output_or_default(output.take());
}

pub(super) fn apply_backup_dir_default(runtime: &AppRuntime, backup_dir: &mut Option<PathBuf>) {
    *backup_dir = runtime.backup_dir_or_default(backup_dir.take());
}

pub(super) fn apply_source_platform_default(
    runtime: &AppRuntime,
    source_platform: &mut Option<HostPlatformValue>,
) {
    *source_platform = Some(runtime.source_platform_or_host(source_platform.take()));
}

#[cfg(test)]
mod tests;
