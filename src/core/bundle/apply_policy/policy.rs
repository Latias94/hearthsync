use super::super::types::apply::ApplyGroup;
use crate::core::manifest::{BundleManifest, ResourceApplyPolicy};

pub(in crate::core::bundle) fn resource_policy_for_group(
    manifest: &BundleManifest,
    group: ApplyGroup,
) -> ResourceApplyPolicy {
    match group {
        ApplyGroup::Addons => manifest.apply.addons,
        ApplyGroup::WtfCommon => manifest.apply.wtf_common,
        ApplyGroup::WtfCharacters => manifest.apply.wtf_characters,
        ApplyGroup::Fonts => manifest.apply.fonts,
        ApplyGroup::InterfaceAssets => manifest.apply.interface_assets,
        ApplyGroup::Metadata => ResourceApplyPolicy::Merge,
    }
}
