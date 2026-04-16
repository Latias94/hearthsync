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
pub use service::{inspect_installation, resolve_installation, scan_installations};
