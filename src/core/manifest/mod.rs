mod example;
mod io;
mod model;

#[cfg(test)]
mod tests;

pub use example::example_manifest;
pub use io::load_manifest;
pub use model::{
    ApplyDefaults, BundleManifest, BundleResources, CharacterMappingMode, CharacterResource,
    MappingRules, PackageMetadata, ResourceApplyPolicy, SourceInstallation,
};
