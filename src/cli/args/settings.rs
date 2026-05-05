use std::path::PathBuf;

use clap::Subcommand;

use super::shared::{AddonCacheRepairRemotePolicyArg, AddonStateStorageArg};

#[derive(Debug, Subcommand)]
pub enum SettingsCommands {
    Inspect,
    Set {
        #[arg(long, value_enum)]
        addon_state_storage: Option<AddonStateStorageArg>,
        #[arg(long, conflicts_with = "addon_state_storage")]
        clear_addon_state_storage: bool,
        #[arg(long)]
        addon_cache_dir: Option<PathBuf>,
        #[arg(long, conflicts_with = "addon_cache_dir")]
        clear_addon_cache_dir: bool,
        #[arg(
            long,
            conflicts_with_all = ["addon_http_no_validator_window_secs", "clear_addon_http_no_validator_policy"]
        )]
        addon_http_no_validator_always_refresh: bool,
        #[arg(
            long,
            value_parser = clap::value_parser!(u64).range(1..),
            conflicts_with = "clear_addon_http_no_validator_policy"
        )]
        addon_http_no_validator_window_secs: Option<u64>,
        #[arg(
            long,
            conflicts_with_all = ["addon_http_no_validator_always_refresh", "addon_http_no_validator_window_secs"]
        )]
        clear_addon_http_no_validator_policy: bool,
        #[arg(
            long,
            value_enum,
            conflicts_with = "clear_addon_cache_repair_remote_policy"
        )]
        addon_cache_repair_remote_policy: Option<AddonCacheRepairRemotePolicyArg>,
        #[arg(long, conflicts_with = "addon_cache_repair_remote_policy")]
        clear_addon_cache_repair_remote_policy: bool,
    },
    Reset,
}
