use std::fs::{self, File};
use std::path::Path;

use zip::ZipArchive;

use super::super::archive_read::inspect::{count_bundle_entries, read_manifest_from_archive};
use super::super::types::apply::BundleApplyMappings;
use super::super::types::archive::BundleInspection;
use crate::core::error::AppResult;

pub fn inspect_bundle(path: &Path) -> AppResult<BundleInspection> {
    let archive_path = path.to_path_buf();
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let manifest = read_manifest_from_archive(&mut archive)?;
    manifest.validate()?;
    let entries = count_bundle_entries(&mut archive)?;

    Ok(BundleInspection {
        archive_path,
        manifest,
        entries,
    })
}

pub fn load_apply_mappings(path: &Path) -> AppResult<BundleApplyMappings> {
    let content = fs::read_to_string(path)?;
    let mappings = toml::from_str::<BundleApplyMappings>(&content)?;
    mappings.validate()?;
    Ok(mappings)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::load_apply_mappings;

    #[test]
    fn load_apply_mappings_rejects_invalid_file_contracts() {
        let temp = tempdir().expect("temp dir");
        let mapping_path = temp.path().join("mapping.toml");
        fs::write(
            &mapping_path,
            r#"
selected_accounts = ["AccountA", "accounta"]
"#,
        )
        .expect("mapping file");

        let error = load_apply_mappings(&mapping_path)
            .expect_err("duplicate selected accounts should fail");

        assert!(
            error
                .to_string()
                .contains("duplicate selected account mapping")
        );
    }

    use std::fs;
}
