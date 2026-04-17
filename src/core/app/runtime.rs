use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::addon::{AddonProvider, AddonProviderOptions, DefaultAddonProvider};
use crate::core::install::HostPlatform;

pub type SharedAddonProvider = Arc<dyn AddonProvider + Send + Sync>;

#[derive(Clone)]
pub struct AppRuntime {
    addon_provider: SharedAddonProvider,
    host_platform: HostPlatform,
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
            host_platform: HostPlatform::current(),
            install_scan_roots: None,
            default_backup_dir: None,
            default_bundle_output_dir: None,
        }
    }

    pub fn addon_provider(&self) -> &(dyn AddonProvider + Send + Sync) {
        self.addon_provider.as_ref()
    }

    pub fn with_host_platform(mut self, host_platform: HostPlatform) -> Self {
        self.host_platform = host_platform;
        self
    }

    pub fn host_platform(&self) -> HostPlatform {
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
