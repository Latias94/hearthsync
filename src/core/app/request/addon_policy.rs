use crate::core::addon::policy::{
    RemoveAddonPolicyRequest as DomainRemoveAddonPolicyRequest,
    SetAddonPolicyRequest as DomainSetAddonPolicyRequest,
};
use crate::core::app::AppRuntime;
use crate::core::app::{AddonPolicyPinValue, AddonReleaseChannelValue, ResolvedInstallationValue};
use crate::core::error::AppResult;
use crate::core::install::DetectedFlavorInstallation;

#[derive(Debug, Clone)]
pub struct InspectAddonPolicyRequest {
    pub installation: ResolvedInstallationValue,
}

impl InspectAddonPolicyRequest {
    pub(crate) fn into_domain_inputs(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<(
        DetectedFlavorInstallation,
        crate::core::addon::AddonStatePaths,
    )> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;
        Ok((installation, state_paths))
    }
}

#[derive(Debug, Clone)]
pub struct SetAddonPolicyAppRequest {
    pub installation: ResolvedInstallationValue,
    pub package: String,
    pub ignored: Option<bool>,
    pub pin: Option<AddonPolicyPinValue>,
    pub release_channel: Option<AddonReleaseChannelValue>,
    pub allow_prerelease: Option<bool>,
    pub install_dependencies: Option<bool>,
}

impl SetAddonPolicyAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainSetAddonPolicyRequest> {
        let (pinned_version, pinned_file_id) = match self.pin.map(AddonPolicyPinValue::into_domain)
        {
            Some(crate::core::addon::policy::AddonPolicyPin::Version { value }) => {
                (Some(value), None)
            }
            Some(crate::core::addon::policy::AddonPolicyPin::FileId { value }) => {
                (None, Some(value))
            }
            None => (None, None),
        };
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainSetAddonPolicyRequest {
            installation,
            state_paths,
            package: self.package,
            ignored: self.ignored,
            pinned_version,
            pinned_file_id,
            release_channel: self
                .release_channel
                .map(AddonReleaseChannelValue::into_domain),
            allow_prerelease: self.allow_prerelease,
            install_dependencies: self.install_dependencies,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RemoveAddonPolicyAppRequest {
    pub installation: ResolvedInstallationValue,
    pub package: String,
}

impl RemoveAddonPolicyAppRequest {
    pub(crate) fn into_domain_request(
        self,
        runtime: &AppRuntime,
    ) -> AppResult<DomainRemoveAddonPolicyRequest> {
        let installation = self.installation.into_domain();
        let state_paths = runtime.addon_state_paths(&installation)?;

        Ok(DomainRemoveAddonPolicyRequest {
            installation,
            state_paths,
            package: self.package,
        })
    }
}
