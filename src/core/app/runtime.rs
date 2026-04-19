use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::addon::{AddonProvider, AddonProviderOptions, DefaultAddonProvider};

use super::HostPlatformValue;

pub type SharedAddonProvider = Arc<dyn AddonProvider + Send + Sync>;

#[derive(Clone)]
pub struct AppRuntime {
    addon_provider: SharedAddonProvider,
    host_platform: HostPlatformValue,
    install_scan_roots: Option<Vec<PathBuf>>,
    default_backup_dir: Option<PathBuf>,
    default_bundle_output_dir: Option<PathBuf>,
}

impl AppRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_addon_provider_options(options: AddonProviderOptions) -> Self {
        Self::with_addon_provider(DefaultAddonProvider::default().with_options(options))
    }

    pub fn with_addon_provider<P>(provider: P) -> Self
    where
        P: AddonProvider + Send + Sync + 'static,
    {
        Self {
            addon_provider: Arc::new(provider),
            host_platform: HostPlatformValue::current(),
            install_scan_roots: None,
            default_backup_dir: None,
            default_bundle_output_dir: None,
        }
    }

    pub fn addon_provider(&self) -> &(dyn AddonProvider + Send + Sync) {
        self.addon_provider.as_ref()
    }

    pub fn with_host_platform(mut self, host_platform: HostPlatformValue) -> Self {
        self.host_platform = host_platform;
        self
    }

    pub fn host_platform(&self) -> HostPlatformValue {
        self.host_platform
    }

    pub fn with_install_scan_roots(mut self, install_scan_roots: Option<Vec<PathBuf>>) -> Self {
        self.install_scan_roots = install_scan_roots;
        self
    }

    pub fn install_scan_roots(&self) -> Option<&[PathBuf]> {
        self.install_scan_roots.as_deref()
    }

    pub fn with_default_backup_dir(mut self, default_backup_dir: Option<PathBuf>) -> Self {
        self.default_backup_dir = default_backup_dir;
        self
    }

    pub fn default_backup_dir(&self) -> Option<&Path> {
        self.default_backup_dir.as_deref()
    }

    pub fn backup_output_or_default(&self, path: Option<PathBuf>) -> Option<PathBuf> {
        path.or_else(|| self.default_backup_dir.clone())
    }

    pub fn backup_dir_or_default(&self, path: Option<PathBuf>) -> Option<PathBuf> {
        path.or_else(|| self.default_backup_dir.clone())
    }

    pub fn with_default_bundle_output_dir(
        mut self,
        default_bundle_output_dir: Option<PathBuf>,
    ) -> Self {
        self.default_bundle_output_dir = default_bundle_output_dir;
        self
    }

    pub fn default_bundle_output_dir(&self) -> Option<&Path> {
        self.default_bundle_output_dir.as_deref()
    }

    pub fn bundle_output_or_default(&self, path: Option<PathBuf>) -> Option<PathBuf> {
        path.or_else(|| self.default_bundle_output_dir.clone())
    }

    pub fn source_platform_or_host(
        &self,
        platform: Option<HostPlatformValue>,
    ) -> HostPlatformValue {
        platform.unwrap_or(self.host_platform)
    }
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::with_addon_provider(DefaultAddonProvider::default())
    }
}

impl fmt::Debug for AppRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppRuntime")
            .field("host_platform", &self.host_platform)
            .field("install_scan_roots", &self.install_scan_roots)
            .field("default_backup_dir", &self.default_backup_dir)
            .field("default_bundle_output_dir", &self.default_bundle_output_dir)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::core::app::HostPlatformValue;

    #[test]
    fn runtime_default_helpers_preserve_explicit_paths_and_fill_missing_ones() {
        let backup_dir = PathBuf::from("backups");
        let bundle_dir = PathBuf::from("bundles");
        let explicit_backup = PathBuf::from("custom-backups");
        let explicit_bundle = PathBuf::from("custom-bundles");
        let runtime = AppRuntime::new()
            .with_default_backup_dir(Some(backup_dir.clone()))
            .with_default_bundle_output_dir(Some(bundle_dir.clone()));

        assert_eq!(
            runtime.backup_output_or_default(None),
            Some(backup_dir.clone())
        );
        assert_eq!(
            runtime.backup_output_or_default(Some(explicit_backup.clone())),
            Some(explicit_backup)
        );
        assert_eq!(runtime.backup_dir_or_default(None), Some(backup_dir));
        assert_eq!(
            runtime.bundle_output_or_default(None),
            Some(bundle_dir.clone())
        );
        assert_eq!(
            runtime.bundle_output_or_default(Some(explicit_bundle.clone())),
            Some(explicit_bundle)
        );
    }

    #[test]
    fn runtime_source_platform_or_host_uses_explicit_platform_before_host_default() {
        let runtime = AppRuntime::new().with_host_platform(HostPlatformValue::MacOs);

        assert_eq!(
            runtime.source_platform_or_host(None),
            HostPlatformValue::MacOs
        );
        assert_eq!(
            runtime.source_platform_or_host(Some(HostPlatformValue::Windows)),
            HostPlatformValue::Windows
        );
    }
}
