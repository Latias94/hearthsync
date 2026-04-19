use crate::core::error::AppResult;
use crate::core::install::{
    inspect_installation_on_host, resolve_installation_on_host, scan_installations_for_host,
    scan_installations_with_roots,
};

use super::{
    AppRuntime, InspectInstallationRequest, InstallationInspectionResult, InstallationScanResult,
    ResolveInstallationRequest, ResolvedInstallationValue,
};

#[derive(Debug, Clone, Default)]
pub struct InstallationService {
    runtime: AppRuntime,
}

impl InstallationService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn scan(&self) -> AppResult<InstallationScanResult> {
        let installations = match self.runtime.install_scan_roots() {
            Some(roots) => {
                scan_installations_with_roots(roots, self.runtime.host_platform().into())
            }
            None => scan_installations_for_host(self.runtime.host_platform().into()),
        }?;

        Ok(InstallationScanResult::from_installations(installations))
    }

    pub fn inspect(
        &self,
        request: InspectInstallationRequest,
    ) -> AppResult<InstallationInspectionResult> {
        let inspection = inspect_installation_on_host(
            &request.path,
            request.flavor.map(Into::into),
            self.runtime.host_platform().into(),
        )?;
        Ok(InstallationInspectionResult::from(inspection))
    }

    pub fn resolve(
        &self,
        request: ResolveInstallationRequest,
    ) -> AppResult<ResolvedInstallationValue> {
        let installation = resolve_installation_on_host(
            &request.path,
            request.flavor.map(Into::into),
            self.runtime.host_platform().into(),
        )?;
        Ok(ResolvedInstallationValue::from(installation))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::core::app::{
        AddonService, HealthStatusValue, HostPlatformValue, ListAddonsRequest, WowFlavorValue,
    };

    #[test]
    fn installation_service_scan_uses_runtime_scan_roots_and_host_platform() {
        let temp = tempdir().expect("temp dir");
        let product_root = temp.path().join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");

        fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
        fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");

        let service = InstallationService::with_runtime(
            AppRuntime::new()
                .with_host_platform(HostPlatformValue::MacOs)
                .with_install_scan_roots(Some(vec![product_root.clone()])),
        );
        let installations = service.scan().expect("scan installations");

        assert_eq!(installations.installation_count, 1);
        assert_eq!(
            installations.installations[0].platform,
            HostPlatformValue::MacOs
        );
        assert_eq!(installations.installations[0].product_root, product_root);
    }

    #[test]
    fn installation_service_inspect_and_resolve_use_runtime_host_platform() {
        let temp = tempdir().expect("temp dir");
        let product_root = temp.path().join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");

        fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
        fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");
        fs::write(
            flavor_root.join("WTF").join("Config.wtf"),
            "SET locale enUS",
        )
        .expect("config");

        let service = InstallationService::with_runtime(
            AppRuntime::new().with_host_platform(HostPlatformValue::MacOs),
        );
        let inspection = service
            .inspect(InspectInstallationRequest {
                path: product_root.clone(),
                flavor: Some(WowFlavorValue::Retail),
            })
            .expect("inspect");
        let resolved = service
            .resolve(ResolveInstallationRequest {
                path: product_root,
                flavor: Some(WowFlavorValue::Retail),
            })
            .expect("resolve");

        assert_eq!(inspection.installation.platform, HostPlatformValue::MacOs);
        assert_eq!(inspection.health.status, HealthStatusValue::Warning);
        assert_eq!(resolved.platform, HostPlatformValue::MacOs);
        assert!(
            resolved
                .flavor_root
                .ends_with(Path::new("World of Warcraft").join("_retail_"))
        );

        let inventory = AddonService::new()
            .list(ListAddonsRequest {
                installation: resolved,
            })
            .expect("list addons from resolved installation value");
        assert_eq!(inventory.tracked_packages.len(), 0);
    }
}
