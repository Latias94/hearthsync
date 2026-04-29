use std::path::Path;

use crate::core::error::AppResult;

use super::BundleManifest;

pub fn load_manifest(path: &Path) -> AppResult<BundleManifest> {
    let content = std::fs::read_to_string(path)?;
    let manifest: BundleManifest = toml::from_str(&content)?;
    manifest.validate()?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::load_manifest;
    use crate::core::manifest::{BundleManifest, example_manifest};

    #[test]
    fn load_manifest_rejects_invalid_manifest_contracts() {
        let temp = tempdir().expect("temp dir");
        let manifest_path = temp.path().join("manifest.toml");
        let mut manifest: BundleManifest =
            toml::from_str(&example_manifest().expect("example")).expect("parse example");
        manifest.schema_version = 0;
        fs::write(
            &manifest_path,
            toml::to_string_pretty(&manifest).expect("manifest"),
        )
        .expect("write manifest");

        let error = load_manifest(&manifest_path).expect_err("invalid manifest should fail closed");

        assert!(
            error
                .to_string()
                .contains("schema_version must be greater than zero")
        );
    }
}
