use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::addon::{AddonProvider, DefaultAddonProvider};

use super::{AddonProviderOptionsValue, HostPlatformValue};

type SharedAddonProvider = Arc<dyn AddonProvider + Send + Sync>;

#[derive(Clone)]
pub struct AppRuntime {
    addon_provider: SharedAddonProvider,
    default_addon_provider_options: Option<AddonProviderOptionsValue>,
    host_platform: HostPlatformValue,
    install_scan_roots: Option<Vec<PathBuf>>,
    default_backup_dir: Option<PathBuf>,
    default_bundle_output_dir: Option<PathBuf>,
}

impl AppRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_addon_provider_options(options: AddonProviderOptionsValue) -> Self {
        Self {
            addon_provider: Arc::new(
                DefaultAddonProvider::default().with_options(options.clone().into()),
            ),
            default_addon_provider_options: Some(options),
            host_platform: HostPlatformValue::current(),
            install_scan_roots: None,
            default_backup_dir: None,
            default_bundle_output_dir: None,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_addon_provider<P>(provider: P) -> Self
    where
        P: AddonProvider + Send + Sync + 'static,
    {
        Self {
            addon_provider: Arc::new(provider),
            default_addon_provider_options: None,
            host_platform: HostPlatformValue::current(),
            install_scan_roots: None,
            default_backup_dir: None,
            default_bundle_output_dir: None,
        }
    }

    pub(crate) fn addon_provider(&self) -> &(dyn AddonProvider + Send + Sync) {
        self.addon_provider.as_ref()
    }

    pub fn default_addon_provider_options(&self) -> Option<&AddonProviderOptionsValue> {
        self.default_addon_provider_options.as_ref()
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
        Self::with_addon_provider_options(AddonProviderOptionsValue::default())
    }
}

impl fmt::Debug for AppRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppRuntime")
            .field(
                "default_addon_provider_options",
                &self.default_addon_provider_options,
            )
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
    use crate::core::app::{AddonProviderRetryPolicyValue, HostPlatformValue};

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

    #[test]
    fn runtime_exposes_default_addon_provider_options_for_default_provider() {
        let runtime = AppRuntime::with_addon_provider_options(AddonProviderOptionsValue {
            download_cache_dir: Some(PathBuf::from("cache")),
            retry_policy: AddonProviderRetryPolicyValue { max_attempts: 3 },
        });

        assert_eq!(
            runtime.default_addon_provider_options(),
            Some(&AddonProviderOptionsValue {
                download_cache_dir: Some(PathBuf::from("cache")),
                retry_policy: AddonProviderRetryPolicyValue { max_attempts: 3 },
            })
        );
    }

    #[test]
    fn runtime_clears_default_provider_options_when_custom_provider_is_injected() {
        let runtime = AppRuntime::with_addon_provider(DefaultAddonProvider::default());

        assert!(runtime.default_addon_provider_options().is_none());
    }
}
