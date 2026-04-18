use crate::core::error::AppResult;
use crate::core::install::{
    DetectedFlavorInstallation, ProductInstallInspection, inspect_installation_on_host,
    resolve_installation_on_host, scan_installations_for_host, scan_installations_with_roots,
};

use super::{AppRuntime, InspectInstallationRequest, ResolveInstallationRequest};

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

    pub fn scan(&self) -> AppResult<Vec<DetectedFlavorInstallation>> {
        match self.runtime.install_scan_roots() {
            Some(roots) => scan_installations_with_roots(roots, self.runtime.host_platform()),
            None => scan_installations_for_host(self.runtime.host_platform()),
        }
    }

    pub fn inspect(
        &self,
        request: InspectInstallationRequest,
    ) -> AppResult<ProductInstallInspection> {
        inspect_installation_on_host(&request.path, request.flavor, self.runtime.host_platform())
    }

    pub fn resolve(
        &self,
        request: ResolveInstallationRequest,
    ) -> AppResult<DetectedFlavorInstallation> {
        resolve_installation_on_host(&request.path, request.flavor, self.runtime.host_platform())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use super::*;
    use crate::core::install::{HealthStatus, HostPlatform, WowFlavor};

    #[test]
    fn installation_service_scan_uses_runtime_scan_roots_and_host_platform() {
        let temp = tempdir().expect("temp dir");
        let product_root = temp.path().join("World of Warcraft");
        let flavor_root = product_root.join("_retail_");

        fs::create_dir_all(flavor_root.join("Interface").join("AddOns")).expect("addons dir");
        fs::create_dir_all(flavor_root.join("WTF")).expect("wtf dir");

        let service = InstallationService::with_runtime(
            AppRuntime::new()
                .with_host_platform(HostPlatform::MacOs)
                .with_install_scan_roots(Some(vec![product_root.clone()])),
        );
        let installations = service.scan().expect("scan installations");

        assert_eq!(installations.len(), 1);
        assert_eq!(installations[0].platform, HostPlatform::MacOs);
        assert_eq!(installations[0].product_root, product_root);
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
            AppRuntime::new().with_host_platform(HostPlatform::MacOs),
        );
        let inspection = service
            .inspect(InspectInstallationRequest {
                path: product_root.clone(),
                flavor: Some(WowFlavor::Retail),
            })
            .expect("inspect");
        let resolved = service
            .resolve(ResolveInstallationRequest {
                path: product_root,
                flavor: Some(WowFlavor::Retail),
            })
            .expect("resolve");

        assert_eq!(inspection.installation.platform, HostPlatform::MacOs);
        assert_eq!(inspection.health.status, HealthStatus::Warning);
        assert_eq!(resolved.platform, HostPlatform::MacOs);
        assert!(
            resolved
                .flavor_root
                .ends_with(Path::new("World of Warcraft").join("_retail_"))
        );
    }
}
