use clap::ValueEnum;

use crate::core::app::{HostPlatformValue, WowFlavorValue};
use crate::core::manifest::ResourceApplyPolicy;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum FlavorArg {
    Retail,
    Classic,
    #[value(name = "classic-era")]
    ClassicEra,
    Ptr,
    Beta,
    Xptr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PlatformArg {
    Windows,
    #[value(name = "macos")]
    MacOs,
    Linux,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ApplyPolicyArg {
    Merge,
    Share,
    Sync,
    Mirror,
    #[value(name = "replace-selected")]
    ReplaceSelected,
    Preserve,
}

impl From<FlavorArg> for WowFlavorValue {
    fn from(value: FlavorArg) -> Self {
        match value {
            FlavorArg::Retail => WowFlavorValue::Retail,
            FlavorArg::Classic => WowFlavorValue::Classic,
            FlavorArg::ClassicEra => WowFlavorValue::ClassicEra,
            FlavorArg::Ptr => WowFlavorValue::Ptr,
            FlavorArg::Beta => WowFlavorValue::Beta,
            FlavorArg::Xptr => WowFlavorValue::Xptr,
        }
    }
}

impl From<PlatformArg> for HostPlatformValue {
    fn from(value: PlatformArg) -> Self {
        match value {
            PlatformArg::Windows => HostPlatformValue::Windows,
            PlatformArg::MacOs => HostPlatformValue::MacOs,
            PlatformArg::Linux => HostPlatformValue::Linux,
            PlatformArg::Unknown => HostPlatformValue::Unknown,
        }
    }
}

impl From<ApplyPolicyArg> for ResourceApplyPolicy {
    fn from(value: ApplyPolicyArg) -> Self {
        match value {
            ApplyPolicyArg::Merge => ResourceApplyPolicy::Merge,
            ApplyPolicyArg::Share => ResourceApplyPolicy::Share,
            ApplyPolicyArg::Sync => ResourceApplyPolicy::Sync,
            ApplyPolicyArg::Mirror => ResourceApplyPolicy::Mirror,
            ApplyPolicyArg::ReplaceSelected => ResourceApplyPolicy::ReplaceSelected,
            ApplyPolicyArg::Preserve => ResourceApplyPolicy::Preserve,
        }
    }
}
