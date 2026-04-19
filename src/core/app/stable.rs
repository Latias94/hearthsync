use super::{
    AddonService, AppRuntime, BackupService, BundleService, ExternalPackageService,
    InstallationService,
};

#[derive(Debug, Clone, Default)]
pub struct StableAppServices {
    runtime: AppRuntime,
}

impl StableAppServices {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn installations(&self) -> InstallationService {
        InstallationService::with_runtime(self.runtime.clone())
    }

    pub fn addons(&self) -> AddonService {
        AddonService::with_runtime(self.runtime.clone())
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
    use crate::core::app::HostPlatformValue;

    #[test]
    fn stable_app_services_share_runtime_with_first_wave_gui_services() {
        let temp = tempdir().expect("temp dir");
        let scan_root = temp.path().join("scan-root");
        let backup_dir = temp.path().join("backups");
        let bundle_dir = temp.path().join("bundles");
        let runtime = AppRuntime::new()
            .with_host_platform(HostPlatformValue::MacOs)
            .with_install_scan_roots(Some(vec![scan_root.clone()]))
            .with_default_backup_dir(Some(backup_dir.clone()))
            .with_default_bundle_output_dir(Some(bundle_dir.clone()));

        let services = StableAppServices::with_runtime(runtime);

        assert_eq!(
            services.installations().runtime().install_scan_roots(),
            Some([scan_root].as_slice())
        );
        assert_eq!(
            services.installations().runtime().host_platform(),
            HostPlatformValue::MacOs
        );
        assert_eq!(
            services.backups().runtime().default_backup_dir(),
            Some(backup_dir.as_path())
        );
        assert_eq!(
            services.bundles().runtime().default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(
            services
                .external_packages()
                .runtime()
                .default_bundle_output_dir(),
            Some(bundle_dir.as_path())
        );
        assert_eq!(
            services.addons().runtime().host_platform(),
            HostPlatformValue::MacOs
        );
    }
}
