use super::{
    AddonIndexService, AddonLockService, AddonService, AppRuntime, BackupService, BundleService,
    ExternalPackageService, InstallationService, StableAppServices,
};

#[derive(Debug, Clone, Default)]
pub struct HearthSyncApp {
    runtime: AppRuntime,
}

impl HearthSyncApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn stable_services(&self) -> StableAppServices {
        StableAppServices::with_runtime(self.runtime.clone())
    }

    pub fn installations(&self) -> InstallationService {
        InstallationService::with_runtime(self.runtime.clone())
    }

    pub fn addons(&self) -> AddonService {
        AddonService::with_runtime(self.runtime.clone())
    }

    pub fn addon_indexes(&self) -> AddonIndexService {
        AddonIndexService::with_runtime(self.runtime.clone())
    }

    pub fn addon_locks(&self) -> AddonLockService {
        AddonLockService::with_runtime(self.runtime.clone())
    }

    pub fn backups(&self) -> BackupService {
        BackupService::with_runtime(self.runtime.clone())
    }

    pub fn bundles(&self) -> BundleService {
        BundleService::with_runtime(self.runtime.clone())
    }

    pub fn external_packages(&self) -> ExternalPackageService {
        ExternalPackageService::with_runtime(self.runtime.clone())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::core::install::HostPlatform;

    #[test]
    fn hearthsync_app_builds_services_with_shared_runtime() {
        let temp = tempdir().expect("temp dir");
        let scan_root = temp.path().join("scan-root");
        let backup_dir = temp.path().join("backups");
        let bundle_dir = temp.path().join("bundles");
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatform::MacOs)
            .with_install_scan_roots(Some(vec![scan_root.clone()]))
            .with_default_backup_dir(Some(backup_dir.clone()))
            .with_default_bundle_output_dir(Some(bundle_dir.clone()));

        let app = HearthSyncApp::with_runtime(runtime);

        assert_eq!(
            app.installations().runtime().install_scan_roots(),
            Some([scan_root].as_slice())
        );
        assert_eq!(
            app.installations().runtime().host_platform(),
            HostPlatform::MacOs
        );
        assert_eq!(
            app.backups().runtime().default_backup_dir(),
            Some(backup_dir.as_path())
        );
        assert_eq!(
            app.bundles().runtime().default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(
            app.external_packages()
                .runtime()
                .default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(app.addons().runtime().host_platform(), HostPlatform::MacOs);
        assert_eq!(
            app.addon_indexes().runtime().host_platform(),
            HostPlatform::MacOs
        );
        assert_eq!(
            app.addon_locks().runtime().host_platform(),
            HostPlatform::MacOs
        );
    }

    #[test]
    fn hearthsync_app_exposes_first_wave_stable_services() {
        let temp = tempdir().expect("temp dir");
        let backup_dir = temp.path().join("backups");
        let bundle_dir = temp.path().join("bundles");
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatform::MacOs)
            .with_default_backup_dir(Some(backup_dir.clone()))
            .with_default_bundle_output_dir(Some(bundle_dir.clone()));

        let app = HearthSyncApp::with_runtime(runtime);
        let stable = app.stable_services();

        assert_eq!(stable.runtime().host_platform(), HostPlatform::MacOs);
        assert_eq!(
            stable.backups().runtime().default_backup_dir(),
            Some(backup_dir.as_path())
        );
        assert_eq!(
            stable.bundles().runtime().default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(
            stable.addons().runtime().host_platform(),
            HostPlatform::MacOs
        );
    }
}
