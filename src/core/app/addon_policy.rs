use crate::core::addon::policy::{inspect_addon_policy, remove_addon_policy, set_addon_policy};
use crate::core::app::{
    AddonPolicyInspectionResult, AddonPolicyMutationResult, AppRuntime, InspectAddonPolicyRequest,
    RemoveAddonPolicyAppRequest, SetAddonPolicyAppRequest,
};
use crate::core::error::AppResult;

#[derive(Debug, Clone, Default)]
pub(super) struct AddonPolicyService {
    #[allow(dead_code)]
    runtime: AppRuntime,
}

impl AddonPolicyService {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn with_runtime(runtime: AppRuntime) -> Self {
        Self { runtime }
    }

    #[cfg(test)]
    pub(super) fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub(super) fn inspect(
        &self,
        request: InspectAddonPolicyRequest,
    ) -> AppResult<AddonPolicyInspectionResult> {
        let (installation, state_paths) = request.into_domain_inputs(&self.runtime)?;
        let inspection = inspect_addon_policy(&installation, &state_paths)?;
        Ok(AddonPolicyInspectionResult::from_domain(inspection))
    }

    pub(super) fn set(
        &self,
        request: SetAddonPolicyAppRequest,
    ) -> AppResult<AddonPolicyMutationResult> {
        let result = set_addon_policy(request.into_domain_request(&self.runtime)?)?;
        Ok(AddonPolicyMutationResult::from_domain(result))
    }

    pub(super) fn remove(
        &self,
        request: RemoveAddonPolicyAppRequest,
    ) -> AppResult<AddonPolicyMutationResult> {
        let result = remove_addon_policy(request.into_domain_request(&self.runtime)?)?;
        Ok(AddonPolicyMutationResult::from_domain(result))
    }
}
#[cfg(test)]
mod tests;
