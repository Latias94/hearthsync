use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::addon::{AddonProvider, DefaultAddonProvider};
use crate::core::error::AppResult;
use crate::core::install::{
    DetectedFlavorInstallation, scan_installations_for_host, scan_installations_with_roots,
};

use super::{
    AddonProviderModeValue, AddonProviderOptionsValue, AppRuntimeCapabilitiesValue,
    ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue, ExternalHelperPolicyValue,
    HelperStrategyValue, HostPlatformValue,
};

type SharedAddonProvider = Arc<dyn AddonProvider + Send + Sync>;

#[derive(Clone)]
pub struct AppRuntime {
    addon_provider: SharedAddonProvider,
    default_addon_provider_options: Option<AddonProviderOptionsValue>,
    external_helper_policy: ExternalHelperPolicyValue,
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
                DefaultAddonProvider::default().with_options(options.clone().into_domain()),
            ),
            default_addon_provider_options: Some(options),
            external_helper_policy: ExternalHelperPolicyValue::default(),
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
            external_helper_policy: ExternalHelperPolicyValue::default(),
            host_platform: HostPlatformValue::current(),
            install_scan_roots: None,
            default_backup_dir: None,
            default_bundle_output_dir: None,
        }
    }

    pub(crate) fn addon_provider(&self) -> &(dyn AddonProvider + Send + Sync) {
        self.addon_provider.as_ref()
    }

    pub fn with_external_helper_policy(
        mut self,
        external_helper_policy: ExternalHelperPolicyValue,
    ) -> Self {
        self.external_helper_policy = external_helper_policy;
        self
    }

    pub fn capabilities(&self) -> AppRuntimeCapabilitiesValue {
        let addon_provider = match &self.default_addon_provider_options {
            Some(options) => AddonProviderModeValue::ConfiguredDefault {
                options: options.clone(),
            },
            None => AddonProviderModeValue::InternalCustom,
        };

        AppRuntimeCapabilitiesValue {
            addon_provider,
            external_helper: self.external_helper_capabilities(),
        }
    }

    pub fn helper_strategy(&self) -> HelperStrategyValue {
        self.external_helper_capabilities().active_strategy
    }

    pub fn external_helper_policy(&self) -> ExternalHelperPolicyValue {
        self.external_helper_policy
    }

    pub fn external_helper_capabilities(&self) -> ExternalHelperCapabilitiesValue {
        let availability = match self.external_helper_policy {
            ExternalHelperPolicyValue::NativeOnly => ExternalHelperAvailabilityValue::NotRequested,
            ExternalHelperPolicyValue::PreferExternal => {
                ExternalHelperAvailabilityValue::Unavailable
            }
        };

        ExternalHelperCapabilitiesValue {
            policy: self.external_helper_policy,
            availability,
            active_strategy: HelperStrategyValue::NativeRust,
        }
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

    pub(crate) fn scan_installations(&self) -> AppResult<Vec<DetectedFlavorInstallation>> {
        match self.install_scan_roots() {
            Some(roots) => scan_installations_with_roots(roots, self.host_platform.into_domain()),
            None => scan_installations_for_host(self.host_platform.into_domain()),
        }
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
            .field("external_helper_policy", &self.external_helper_policy)
            .field("host_platform", &self.host_platform)
            .field("install_scan_roots", &self.install_scan_roots)
            .field("default_backup_dir", &self.default_backup_dir)
            .field("default_bundle_output_dir", &self.default_bundle_output_dir)
            .finish_non_exhaustive()
    }
}
#[cfg(test)]
mod tests;
