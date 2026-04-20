use std::path::PathBuf;

use crate::core::app::{
    BundleManifestValue, InspectBundleRequest, PackBundleAppRequest, ResolvedInstallationValue,
};
use crate::core::error::AppResult;
use crate::core::manifest::load_manifest;

pub(super) fn build_pack_bundle_request(
    installation: ResolvedInstallationValue,
    manifest_path: PathBuf,
    output_path: Option<PathBuf>,
) -> AppResult<PackBundleAppRequest> {
    let manifest_base_dir = manifest_path.parent().map(|path| path.to_path_buf());
    let manifest = BundleManifestValue::from_domain(load_manifest(&manifest_path)?);

    Ok(PackBundleAppRequest {
        installation,
        manifest,
        output_path,
        manifest_base_dir,
    })
}

pub(super) fn build_inspect_bundle_request(bundle_path: PathBuf) -> InspectBundleRequest {
    InspectBundleRequest { bundle_path }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;
    use crate::cli::test_support::sample_installation;
    use crate::core::manifest::example_manifest;

    #[test]
    fn build_pack_bundle_request_loads_manifest_and_base_dir() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let manifest_dir = temp_dir.path().join("bundle");
        fs::create_dir_all(&manifest_dir).expect("create manifest dir");
        let manifest_path = manifest_dir.join("bundle.toml");
        fs::write(
            &manifest_path,
            example_manifest().expect("example manifest"),
        )
        .expect("write manifest");

        let request = build_pack_bundle_request(
            sample_installation(),
            manifest_path.clone(),
            Some(PathBuf::from("exports")),
        )
        .expect("build request");

        assert_eq!(request.output_path, Some(PathBuf::from("exports")));
        assert_eq!(request.manifest_base_dir, Some(manifest_dir));
        assert_eq!(request.manifest.package.id, "starter-ui-retail");
        assert_eq!(request.manifest.source.supported_targets.len(), 1);
    }

    #[test]
    fn build_inspect_bundle_request_preserves_bundle_path() {
        let request = build_inspect_bundle_request(PathBuf::from("ui.bundle.zip"));

        assert_eq!(request.bundle_path, PathBuf::from("ui.bundle.zip"));
    }
}
