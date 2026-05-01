mod backup;
mod install;
mod remove;
mod update;

pub use install::{install_addon, install_addon_task};
pub use remove::{remove_addons, remove_addons_task};
pub use update::{update_addons, update_addons_task};

pub(crate) use install::{
    InstallAddonExecutionPlan, InstallPreparedAddonRequest, execute_install_plan_task,
    install_addon_task_with_provider, prepare_install_prepared_addon,
};
pub(crate) use update::update_addons_task_with_provider;
