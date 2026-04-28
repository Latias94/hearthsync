use super::app_support::{render_with_value, stable_services};
use super::output::addon::{render_addon_cache_purge, render_addon_cache_repair};
use crate::cli::AddonCacheCommands;
use crate::core::app::AppRuntime;
use crate::core::error::AppResult;

pub(super) fn handle_addon_cache_command(
    json: bool,
    runtime: AppRuntime,
    command: AddonCacheCommands,
) -> AppResult<()> {
    let app = stable_services(runtime);

    match command {
        AddonCacheCommands::Purge => {
            render_with_value(json, || app.purge_addon_cache(), render_addon_cache_purge)
        }
        AddonCacheCommands::Repair => {
            render_with_value(json, || app.repair_addon_cache(), render_addon_cache_repair)
        }
    }
}
