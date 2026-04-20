use std::path::{Path, PathBuf};

use super::super::*;

pub(in crate::core::bundle) fn resolve_addon_index_paths(
    addon_indexes: &[String],
    manifest_base_dir: Option<&Path>,
) -> AppResult<Vec<(String, PathBuf)>> {
    let mut resolved = Vec::new();
    let mut file_names = Vec::new();

    for addon_index in addon_indexes {
        let reference = Path::new(addon_index);
        let source_path = if reference.is_absolute() {
            reference.to_path_buf()
        } else if let Some(base_dir) = manifest_base_dir {
            base_dir.join(reference)
        } else {
            return Err(AppError::Validation(format!(
                "relative addon index path requires `manifest_base_dir`: {addon_index}"
            )));
        };

        if !source_path.is_file() {
            return Err(AppError::NotFound(format!(
                "addon index file does not exist: {}",
                source_path.display()
            )));
        }

        let file_name = source_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "addon index file has no usable file name: {}",
                    source_path.display()
                ))
            })?
            .to_string();
        validate_plain_name("addon index file", &file_name)?;
        if file_names.iter().any(|item| item == &file_name) {
            return Err(AppError::Validation(format!(
                "duplicate addon index file name in bundle metadata: {file_name}"
            )));
        }
        file_names.push(file_name.clone());
        resolved.push((file_name, source_path));
    }

    Ok(resolved)
}
