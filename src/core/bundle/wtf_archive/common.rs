use std::fs;
use std::fs::File;
use std::path::Path;

use zip::ZipWriter;

use super::super::zip_write::add_path_to_zip;
use super::super::*;

pub(in crate::core::bundle) fn add_common_wtf_to_zip(
    zip: &mut ZipWriter<File>,
    wtf_dir: &Path,
) -> AppResult<usize> {
    let mut archived_files = 0usize;
    let config_wtf = wtf_dir.join("Config.wtf");
    if config_wtf.exists() {
        archived_files += add_path_to_zip(zip, &config_wtf, Path::new("wtf/common/Config.wtf"))?;
    }

    let account_root = wtf_dir.join("Account");
    if !account_root.exists() {
        return Ok(archived_files);
    }

    let root_saved_variables = account_root.join("SavedVariables");
    if root_saved_variables.exists() {
        archived_files += add_path_to_zip(
            zip,
            &root_saved_variables,
            &Path::new("wtf/common/root").join("SavedVariables"),
        )?;
    }

    for entry in fs::read_dir(account_root)? {
        let entry = entry?;
        let account_dir = entry.path();
        if !account_dir.is_dir() {
            continue;
        }

        let account_name = entry.file_name().to_string_lossy().to_string();
        if account_name.eq_ignore_ascii_case("SavedVariables") {
            continue;
        }
        validate_plain_name("account", &account_name)?;
        for account_entry in fs::read_dir(&account_dir)? {
            let account_entry = account_entry?;
            let account_path = account_entry.path();
            if !account_path.is_file() {
                continue;
            }

            let file_name = account_entry.file_name().to_string_lossy().to_string();
            validate_plain_name("account WTF file", &file_name)?;
            archived_files += add_path_to_zip(
                zip,
                &account_path,
                &Path::new("wtf/common/accounts")
                    .join(&account_name)
                    .join(file_name),
            )?;
        }

        let saved_variables = account_dir.join("SavedVariables");
        if saved_variables.exists() {
            archived_files += add_path_to_zip(
                zip,
                &saved_variables,
                &Path::new("wtf/common/accounts")
                    .join(account_name)
                    .join("SavedVariables"),
            )?;
        }
    }

    Ok(archived_files)
}
