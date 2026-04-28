use std::path::PathBuf;

use serde::Serialize;

use crate::cli::InstallTargetArgs;
use crate::core::app::{AppRuntime, InspectInstallationRequest};
use crate::core::error::AppResult;
use crate::core::manifest::{example_manifest, load_manifest};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(in crate::cli) struct ManifestExampleResult {
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(in crate::cli) struct ManifestValidationResult {
    pub status: String,
    pub path: PathBuf,
}

pub(super) fn build_inspect_installation_request(
    install_target: InstallTargetArgs,
) -> InspectInstallationRequest {
    InspectInstallationRequest {
        path: install_target.install,
        flavor: install_target.flavor.map(Into::into),
    }
}

pub(super) fn build_manifest_example_result() -> AppResult<ManifestExampleResult> {
    Ok(ManifestExampleResult {
        content: example_manifest()?,
    })
}

pub(super) fn build_manifest_validation_result(
    file: PathBuf,
    runtime: &AppRuntime,
) -> AppResult<ManifestValidationResult> {
    let file = runtime.resolve_input_path(file, "manifest file")?;
    let manifest = load_manifest(&file)?;
    manifest.validate()?;

    Ok(ManifestValidationResult {
        status: "ok".to_string(),
        path: file,
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::{
        build_inspect_installation_request, build_manifest_example_result,
        build_manifest_validation_result,
    };
    use crate::cli::{FlavorArg, InstallTargetArgs};
    use crate::core::app::{AppRuntime, WowFlavorValue};

    #[test]
    fn build_inspect_installation_request_maps_flavor() {
        let request = build_inspect_installation_request(InstallTargetArgs {
            install: PathBuf::from("E:\\Games\\World of Warcraft"),
            flavor: Some(FlavorArg::Retail),
        });

        assert_eq!(request.path, PathBuf::from("E:\\Games\\World of Warcraft"));
        assert_eq!(request.flavor, Some(WowFlavorValue::Retail));
    }

    #[test]
    fn build_manifest_example_result_returns_valid_manifest_content() {
        let result = build_manifest_example_result().expect("example manifest");

        let manifest: crate::core::manifest::BundleManifest =
            toml::from_str(&result.content).expect("parse example manifest");

        manifest.validate().expect("valid example manifest");
    }

    #[test]
    fn build_manifest_validation_result_loads_and_validates_manifest_file() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let manifest_path = temp_dir.path().join("manifest.toml");
        fs::write(
            &manifest_path,
            crate::core::manifest::example_manifest().expect("example manifest"),
        )
        .expect("write manifest");

        let result = build_manifest_validation_result(manifest_path.clone(), &AppRuntime::new())
            .expect("valid manifest");

        assert_eq!(result.status, "ok");
        assert_eq!(result.path, manifest_path);
    }

    #[test]
    fn build_manifest_validation_result_resolves_relative_file_against_runtime_base() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let manifest_path = temp_dir.path().join("manifest.toml");
        fs::write(
            &manifest_path,
            crate::core::manifest::example_manifest().expect("example manifest"),
        )
        .expect("write manifest");
        let runtime = AppRuntime::builder()
            .with_relative_path_base(Some(temp_dir.path().to_path_buf()))
            .build()
            .expect("runtime");

        let result = build_manifest_validation_result(PathBuf::from("manifest.toml"), &runtime)
            .expect("valid manifest");

        assert_eq!(result.status, "ok");
        assert_eq!(result.path, manifest_path);
    }

    #[test]
    fn build_manifest_validation_result_rejects_relative_file_without_runtime_base() {
        let error =
            build_manifest_validation_result(PathBuf::from("manifest.toml"), &AppRuntime::new())
                .expect_err("relative manifest should fail closed");

        assert!(
            error
                .to_string()
                .contains("manifest file relative path requires")
        );
    }
}
