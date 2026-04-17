mod accounts;
mod layout;
mod model;
mod service;

#[cfg(test)]
mod tests;

pub use accounts::discover_local_accounts;
#[allow(unused_imports)]
pub use model::{
    DetectedFlavorInstallation, HealthStatus, HostPlatform, InstallationHealth, LocalWowAccount,
    LocalWowCharacter, ProductInstallInspection, WowFlavor,
};
pub(crate) use service::{
    inspect_installation_on_host, resolve_installation_on_host, scan_installations_for_host,
    scan_installations_with_roots,
};
