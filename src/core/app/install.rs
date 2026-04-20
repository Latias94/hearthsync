use crate::core::error::AppResult;

use super::{
    AppRuntime, InspectInstallationRequest, InstallationInspectionResult, InstallationScanResult,
    ResolveInstallationRequest, ResolvedInstallationValue,
};

#[derive(Debug, Clone, Default)]
pub(super) struct InstallationService {
    runtime: AppRuntime,
}

impl InstallationService {
    pub(super) fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    #[cfg(test)]
    pub(super) fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub(super) fn scan(&self) -> AppResult<InstallationScanResult> {
        let installations = self.runtime.scan_installations()?;
        Ok(InstallationScanResult::from_installations(installations))
    }

    pub(super) fn inspect(
        &self,
        request: InspectInstallationRequest,
    ) -> AppResult<InstallationInspectionResult> {
        let inspection = request.inspect_with_runtime(&self.runtime)?;
        Ok(InstallationInspectionResult::from_domain(inspection))
    }

    pub(super) fn resolve(
        &self,
        request: ResolveInstallationRequest,
    ) -> AppResult<ResolvedInstallationValue> {
        let installation = request.resolve_with_runtime(&self.runtime)?;
        Ok(ResolvedInstallationValue::from_domain(installation))
    }
}
#[cfg(test)]
mod tests;
