use crate::cli::system::{ManifestExampleResult, ManifestValidationResult};

pub(in crate::cli) fn render_manifest_example(item: &ManifestExampleResult) -> String {
    item.content.trim_end().to_string()
}

pub(in crate::cli) fn render_manifest_validation(item: &ManifestValidationResult) -> String {
    format!("Manifest is valid: {}", item.path.display())
}
