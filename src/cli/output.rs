use serde::Serialize;

use crate::core::error::AppResult;

mod addon;
mod addon_lock;
mod backup;
mod bundle;
mod external_package;
mod shared;
mod system;
#[cfg(test)]
mod test_support;

pub(super) use self::addon::{
    render_addon_index_inspection, render_addon_index_install, render_addon_index_update,
    render_addon_install, render_addon_inventory, render_addon_remove, render_addon_search_catalog,
    render_addon_update,
};
pub(super) use self::addon_lock::{
    render_addon_lock_apply, render_addon_lock_diff, render_addon_lock_inspection,
    render_addon_lock_plan, render_addon_lock_verify, render_addon_lock_write,
    render_bundle_addon_lock_apply, render_bundle_addon_lock_plan,
};
pub(super) use self::backup::{
    render_backup_catalog, render_backup_created, render_backup_restored,
};
pub(super) use self::bundle::{
    render_bundle_apply, render_bundle_apply_plan, render_bundle_archive_created,
    render_bundle_archive_inspection,
};
pub(super) use self::external_package::{
    render_external_package_analysis, render_external_package_apply, render_external_package_plan,
};
pub(super) use self::system::{
    render_installation_health_report, render_installation_inspection, render_installation_scan,
};

pub(super) fn render<T, F>(json: bool, value: &T, text_renderer: F) -> AppResult<()>
where
    T: Serialize,
    F: FnOnce(&T) -> String,
{
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{}", text_renderer(value));
    }

    Ok(())
}
