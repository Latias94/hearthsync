use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::core::addon::{
    AddonProvider, AddonStatePaths, AddonStateStorageKind, DefaultAddonProvider,
};
use crate::core::error::{AppError, AppResult};
use crate::core::install::{
    DetectedFlavorInstallation, scan_installations_for_host, scan_installations_with_roots,
};

use super::{
    AddonManagementCapabilitiesValue, AddonProviderModeValue, AddonProviderOptionsValue,
    AddonStatePathsValue, AddonStateStorageValue, AppRuntimeCapabilitiesValue,
    AppRuntimeDiagnosticsValue, ExternalHelperAvailabilityValue, ExternalHelperCapabilitiesValue,
    ExternalHelperPolicyValue, HelperStrategyValue, HostPlatformValue, ResolvedInstallationValue,
};

type SharedAddonProvider = Arc<dyn AddonProvider + Send + Sync>;

#[derive(Clone)]
pub struct AppRuntime {
    addon_provider: SharedAddonProvider,
    default_addon_provider_options: Option<AddonProviderOptionsValue>,
    addon_state_storage_kind: AddonStateStorageKind,
    external_helper_policy: ExternalHelperPolicyValue,
    host_platform: HostPlatformValue,
    install_scan_roots: Option<Vec<PathBuf>>,
    relative_path_base: Option<PathBuf>,
    default_backup_dir: Option<PathBuf>,
    default_bundle_output_dir: Option<PathBuf>,
}

#[derive(Clone)]
enum AppRuntimeAddonProviderConfig {
    Default(AddonProviderOptionsValue),
    Custom(SharedAddonProvider),
}

impl Default for AppRuntimeAddonProviderConfig {
    fn default() -> Self {
        Self::Default(AddonProviderOptionsValue::default())
    }
}

#[derive(Clone)]
pub struct AppRuntimeBuilder {
    addon_provider: AppRuntimeAddonProviderConfig,
    addon_state_storage_kind: AddonStateStorageKind,
    external_helper_policy: ExternalHelperPolicyValue,
    host_platform: HostPlatformValue,
    install_scan_roots: Option<Vec<PathBuf>>,
    relative_path_base: Option<PathBuf>,
    default_backup_dir: Option<PathBuf>,
    default_bundle_output_dir: Option<PathBuf>,
}

impl Default for AppRuntimeBuilder {
    fn default() -> Self {
        Self {
            addon_provider: AppRuntimeAddonProviderConfig::default(),
            addon_state_storage_kind: AddonStateStorageKind::default(),
            external_helper_policy: ExternalHelperPolicyValue::default(),
            host_platform: HostPlatformValue::current(),
            install_scan_roots: None,
            relative_path_base: None,
            default_backup_dir: None,
            default_bundle_output_dir: None,
        }
    }
}

impl AppRuntimeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_addon_provider_options(mut self, options: AddonProviderOptionsValue) -> Self {
        self.addon_provider = AppRuntimeAddonProviderConfig::Default(options);
        self
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_addon_provider<P>(mut self, provider: P) -> Self
    where
        P: AddonProvider + Send + Sync + 'static,
    {
        self.addon_provider = AppRuntimeAddonProviderConfig::Custom(Arc::new(provider));
        self
    }

    pub fn with_addon_state_storage_kind(
        mut self,
        addon_state_storage_kind: AddonStateStorageKind,
    ) -> Self {
        self.addon_state_storage_kind = addon_state_storage_kind;
        self
    }

    pub fn with_external_helper_policy(
        mut self,
        external_helper_policy: ExternalHelperPolicyValue,
    ) -> Self {
        self.external_helper_policy = external_helper_policy;
        self
    }

    pub fn with_host_platform(mut self, host_platform: HostPlatformValue) -> Self {
        self.host_platform = host_platform;
        self
    }

    pub fn with_install_scan_roots(mut self, install_scan_roots: Option<Vec<PathBuf>>) -> Self {
        self.install_scan_roots = install_scan_roots;
        self
    }

    pub fn with_relative_path_base(mut self, relative_path_base: Option<PathBuf>) -> Self {
        self.relative_path_base = relative_path_base;
        self
    }

    pub fn with_default_backup_dir(mut self, default_backup_dir: Option<PathBuf>) -> Self {
        self.default_backup_dir = default_backup_dir;
        self
    }

    pub fn with_default_bundle_output_dir(
        mut self,
        default_bundle_output_dir: Option<PathBuf>,
    ) -> Self {
        self.default_bundle_output_dir = default_bundle_output_dir;
        self
    }

    pub fn build(self) -> AppResult<AppRuntime> {
        let relative_path_base = validate_relative_path_base(self.relative_path_base)?;
        let base = relative_path_base.as_deref();
        let install_scan_roots = resolve_optional_runtime_paths(
            self.install_scan_roots,
            base,
            "installation scan root",
        )?;
        let default_backup_dir = resolve_optional_runtime_path(
            self.default_backup_dir,
            base,
            "default backup directory",
        )?;
        let default_bundle_output_dir = resolve_optional_runtime_path(
            self.default_bundle_output_dir,
            base,
            "default bundle output directory",
        )?;

        let (addon_provider, default_addon_provider_options) = match self.addon_provider {
            AppRuntimeAddonProviderConfig::Default(mut options) => {
                options.download_cache_dir = resolve_optional_runtime_path(
                    options.download_cache_dir,
                    base,
                    "addon cache directory",
                )?;
                (
                    Arc::new(
                        DefaultAddonProvider::default().with_options(options.clone().into_domain()),
                    ) as SharedAddonProvider,
                    Some(options),
                )
            }
            AppRuntimeAddonProviderConfig::Custom(provider) => (provider, None),
        };

        Ok(AppRuntime {
            addon_provider,
            default_addon_provider_options,
            addon_state_storage_kind: self.addon_state_storage_kind,
            external_helper_policy: self.external_helper_policy,
            host_platform: self.host_platform,
            install_scan_roots,
            relative_path_base,
            default_backup_dir,
            default_bundle_output_dir,
        })
    }
}

impl AppRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn builder() -> AppRuntimeBuilder {
        AppRuntimeBuilder::new()
    }

    pub fn with_addon_provider_options(options: AddonProviderOptionsValue) -> AppResult<Self> {
        Self::builder().with_addon_provider_options(options).build()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn with_addon_provider<P>(provider: P) -> Self
    where
        P: AddonProvider + Send + Sync + 'static,
    {
        Self::builder()
            .with_addon_provider(provider)
            .build()
            .expect("custom provider runtime has no fallible path normalization")
    }

    pub(crate) fn addon_provider(&self) -> &(dyn AddonProvider + Send + Sync) {
        self.addon_provider.as_ref()
    }

    pub fn addon_state_storage_kind(&self) -> AddonStateStorageKind {
        self.addon_state_storage_kind
    }

    pub(crate) fn addon_state_paths(
        &self,
        installation: &DetectedFlavorInstallation,
    ) -> AppResult<AddonStatePaths> {
        AddonStatePaths::for_installation(self.addon_state_storage_kind, installation)
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
            addon_management: self.addon_management_capabilities(),
            external_helper: self.external_helper_capabilities(),
        }
    }

    pub fn diagnostics(&self) -> AppRuntimeDiagnosticsValue {
        AppRuntimeDiagnosticsValue {
            host_platform: self.host_platform,
            install_scan_roots: self.install_scan_roots.clone(),
            relative_path_base: self.relative_path_base.clone(),
            default_backup_dir: self.default_backup_dir.clone(),
            default_bundle_output_dir: self.default_bundle_output_dir.clone(),
            selected_installation: None,
            addon_state_paths: None,
            capabilities: self.capabilities(),
        }
    }

    pub fn diagnostics_for_installation(
        &self,
        installation: ResolvedInstallationValue,
    ) -> AppResult<AppRuntimeDiagnosticsValue> {
        let addon_state_paths = self.addon_state_paths_value(&installation)?;

        Ok(AppRuntimeDiagnosticsValue {
            host_platform: self.host_platform,
            install_scan_roots: self.install_scan_roots.clone(),
            relative_path_base: self.relative_path_base.clone(),
            default_backup_dir: self.default_backup_dir.clone(),
            default_bundle_output_dir: self.default_bundle_output_dir.clone(),
            selected_installation: Some(installation),
            addon_state_paths: Some(addon_state_paths),
            capabilities: self.capabilities(),
        })
    }

    pub fn addon_management_capabilities(&self) -> AddonManagementCapabilitiesValue {
        AddonManagementCapabilitiesValue {
            state_storage: AddonStateStorageValue::from_domain(self.addon_state_storage_kind),
            scan_only_without_managed_state: true,
            managed_mode_requires_state: true,
        }
    }

    fn addon_state_paths_value(
        &self,
        installation: &ResolvedInstallationValue,
    ) -> AppResult<AddonStatePathsValue> {
        self.addon_state_paths(&installation.clone().into_domain())
            .map(AddonStatePathsValue::from_domain)
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

    pub fn host_platform(&self) -> HostPlatformValue {
        self.host_platform
    }

    pub fn install_scan_roots(&self) -> Option<&[PathBuf]> {
        self.install_scan_roots.as_deref()
    }

    pub fn relative_path_base(&self) -> Option<&Path> {
        self.relative_path_base.as_deref()
    }

    pub(crate) fn resolve_input_path(
        &self,
        path: PathBuf,
        description: &str,
    ) -> AppResult<PathBuf> {
        self.resolve_runtime_relative_path(path, description)
    }

    pub(crate) fn resolve_output_path(
        &self,
        path: PathBuf,
        description: &str,
    ) -> AppResult<PathBuf> {
        self.resolve_runtime_relative_path(path, description)
    }

    fn resolve_runtime_relative_path(
        &self,
        path: PathBuf,
        description: &str,
    ) -> AppResult<PathBuf> {
        resolve_runtime_path(path, self.relative_path_base.as_deref(), description)
    }

    pub(crate) fn scan_installations(&self) -> AppResult<Vec<DetectedFlavorInstallation>> {
        match self.install_scan_roots() {
            Some(roots) => {
                let roots = roots
                    .iter()
                    .cloned()
                    .map(|root| self.resolve_input_path(root, "installation scan root"))
                    .collect::<AppResult<Vec<_>>>()?;
                scan_installations_with_roots(&roots, self.host_platform.into_domain())
            }
            None => scan_installations_for_host(self.host_platform.into_domain()),
        }
    }

    pub fn default_backup_dir(&self) -> Option<&Path> {
        self.default_backup_dir.as_deref()
    }

    pub(crate) fn backup_output_or_default(&self, path: Option<PathBuf>) -> Option<PathBuf> {
        path.or_else(|| self.default_backup_dir.clone())
    }

    pub(crate) fn backup_dir_or_default(&self, path: Option<PathBuf>) -> Option<PathBuf> {
        path.or_else(|| self.default_backup_dir.clone())
    }

    pub fn default_bundle_output_dir(&self) -> Option<&Path> {
        self.default_bundle_output_dir.as_deref()
    }

    pub(crate) fn bundle_output_or_default(&self, path: Option<PathBuf>) -> Option<PathBuf> {
        path.or_else(|| self.default_bundle_output_dir.clone())
    }

    pub(crate) fn source_platform_or_host(
        &self,
        platform: Option<HostPlatformValue>,
    ) -> HostPlatformValue {
        platform.unwrap_or(self.host_platform)
    }
}

impl Default for AppRuntime {
    fn default() -> Self {
        Self::builder()
            .build()
            .expect("default runtime has no fallible path normalization")
    }
}

fn validate_relative_path_base(relative_path_base: Option<PathBuf>) -> AppResult<Option<PathBuf>> {
    if let Some(base) = relative_path_base.as_deref()
        && !base.is_absolute()
    {
        return Err(AppError::Validation(format!(
            "app runtime relative path base must be absolute: {}",
            base.display()
        )));
    }

    Ok(relative_path_base)
}

fn resolve_optional_runtime_paths(
    paths: Option<Vec<PathBuf>>,
    base: Option<&Path>,
    description: &str,
) -> AppResult<Option<Vec<PathBuf>>> {
    paths
        .map(|paths| {
            paths
                .into_iter()
                .map(|path| resolve_runtime_path(path, base, description))
                .collect()
        })
        .transpose()
}

fn resolve_optional_runtime_path(
    path: Option<PathBuf>,
    base: Option<&Path>,
    description: &str,
) -> AppResult<Option<PathBuf>> {
    path.map(|path| resolve_runtime_path(path, base, description))
        .transpose()
}

fn resolve_runtime_path(
    path: PathBuf,
    base: Option<&Path>,
    description: &str,
) -> AppResult<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }

    let Some(base) = base else {
        return Err(AppError::Validation(format!(
            "{description} relative path requires an app runtime relative path base: {}",
            path.display()
        )));
    };
    if !base.is_absolute() {
        return Err(AppError::Validation(format!(
            "app runtime relative path base must be absolute before resolving {description}: {}",
            base.display()
        )));
    }

    Ok(base.join(path))
}

impl fmt::Debug for AppRuntime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppRuntime")
            .field(
                "default_addon_provider_options",
                &self.default_addon_provider_options,
            )
            .field("addon_state_storage_kind", &self.addon_state_storage_kind)
            .field("external_helper_policy", &self.external_helper_policy)
            .field("host_platform", &self.host_platform)
            .field("install_scan_roots", &self.install_scan_roots)
            .field("relative_path_base", &self.relative_path_base)
            .field("default_backup_dir", &self.default_backup_dir)
            .field("default_bundle_output_dir", &self.default_bundle_output_dir)
            .finish_non_exhaustive()
    }
}
#[cfg(test)]
mod tests;
