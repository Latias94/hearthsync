mod model;
mod service;

#[allow(unused_imports)]
pub use model::{
    DetectedFlavorInstallation, HealthStatus, HostPlatform, InstallationHealth, LocalWowAccount,
    LocalWowCharacter, ProductInstallInspection, WowFlavor,
};
pub use service::{
    discover_local_accounts, inspect_installation, resolve_installation, scan_installations,
};
