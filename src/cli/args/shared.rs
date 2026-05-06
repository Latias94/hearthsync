use std::path::PathBuf;

use clap::{Args, ValueEnum};

use crate::core::addon::AddonStateStorageKind;
use crate::core::app::{
    AddonCacheRepairRemotePolicyValue, AddonReleaseChannelValue, HostPlatformValue,
    HttpNoValidatorCachePolicyValue, WowFlavorValue,
};
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ReleaseChannelArg {
    Stable,
    Beta,
    Alpha,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum AddonStateStorageArg {
    AppData,
    Sidecar,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum AddonCacheRepairRemotePolicyArg {
    #[value(name = "local-only")]
    LocalOnly,
    #[value(name = "validate-remote")]
    ValidateRemote,
    #[value(name = "require-remote")]
    RequireRemote,
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

#[derive(Debug, Clone, Args)]
pub struct InstallTargetArgs {
    #[arg(long, help = "World of Warcraft installation or product root")]
    pub install: PathBuf,
    #[arg(long, value_enum)]
    pub flavor: Option<FlavorArg>,
}

#[derive(Debug, Clone, Args, Default)]
pub struct OptionalInstallTargetArgs {
    #[arg(long, help = "World of Warcraft installation or product root")]
    pub install: Option<PathBuf>,
    #[arg(long, value_enum, requires = "install")]
    pub flavor: Option<FlavorArg>,
}

#[derive(Debug, Clone, Args)]
pub struct ApplyMappingArgs {
    #[arg(long)]
    pub mapping_file: Option<PathBuf>,
    #[arg(long)]
    pub target_account: Option<String>,
    #[arg(long)]
    pub target_server: Option<String>,
    #[arg(long)]
    pub target_character: Option<String>,
    #[arg(long = "select-account")]
    pub selected_accounts: Vec<String>,
    #[arg(long)]
    pub all_accounts: bool,
}

#[derive(Debug, Clone, Args, Default)]
pub struct CliRuntimeArgs {
    #[arg(
        long,
        global = true,
        value_enum,
        help = "Where managed addon state is stored. Defaults to `app-data`; use `sidecar` for portable Interface/AddOns/.hearthsync state"
    )]
    pub addon_state_storage: Option<AddonStateStorageArg>,
    #[arg(
        long,
        global = true,
        help = "Directory for downloaded addon archive cache and cache maintenance commands"
    )]
    pub addon_cache_dir: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        conflicts_with = "addon_http_no_validator_window_secs",
        help = "Disable bounded reuse for HTTP addon archives when the server exposes no reusable validators"
    )]
    pub addon_http_no_validator_always_refresh: bool,
    #[arg(
        long,
        global = true,
        value_parser = clap::value_parser!(u64).range(1..),
        help = "Freshness window in seconds for HTTP addon archives without reusable validators"
    )]
    pub addon_http_no_validator_window_secs: Option<u64>,
    #[arg(
        long,
        global = true,
        value_enum,
        help = "Remote validation policy for addon cache repair"
    )]
    pub addon_cache_repair_remote_policy: Option<AddonCacheRepairRemotePolicyArg>,
    #[arg(
        long,
        global = true,
        value_parser = clap::value_parser!(u64).range(0..),
        help = "Live addon search cache TTL in seconds; set to 0 to disable provider search caching"
    )]
    pub addon_search_cache_ttl_secs: Option<u64>,
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

impl From<ReleaseChannelArg> for AddonReleaseChannelValue {
    fn from(value: ReleaseChannelArg) -> Self {
        match value {
            ReleaseChannelArg::Stable => AddonReleaseChannelValue::Stable,
            ReleaseChannelArg::Beta => AddonReleaseChannelValue::Beta,
            ReleaseChannelArg::Alpha => AddonReleaseChannelValue::Alpha,
        }
    }
}

impl From<AddonStateStorageArg> for AddonStateStorageKind {
    fn from(value: AddonStateStorageArg) -> Self {
        match value {
            AddonStateStorageArg::AppData => AddonStateStorageKind::AppData,
            AddonStateStorageArg::Sidecar => AddonStateStorageKind::Sidecar,
        }
    }
}

impl From<AddonCacheRepairRemotePolicyArg> for AddonCacheRepairRemotePolicyValue {
    fn from(value: AddonCacheRepairRemotePolicyArg) -> Self {
        match value {
            AddonCacheRepairRemotePolicyArg::LocalOnly => Self::LocalOnly,
            AddonCacheRepairRemotePolicyArg::ValidateRemote => Self::ValidateRemote,
            AddonCacheRepairRemotePolicyArg::RequireRemote => Self::RequireRemote,
        }
    }
}

impl CliRuntimeArgs {
    pub fn http_no_validator_cache_policy(&self) -> Option<HttpNoValidatorCachePolicyValue> {
        if self.addon_http_no_validator_always_refresh {
            return Some(HttpNoValidatorCachePolicyValue::AlwaysRefresh);
        }

        self.addon_http_no_validator_window_secs
            .map(|max_age_secs| HttpNoValidatorCachePolicyValue::ReuseWithinWindow { max_age_secs })
    }

    pub fn cache_repair_remote_policy(&self) -> Option<AddonCacheRepairRemotePolicyValue> {
        self.addon_cache_repair_remote_policy.map(Into::into)
    }
}
