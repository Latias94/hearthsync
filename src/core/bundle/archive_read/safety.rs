use crate::core::error::{AppError, AppResult};

pub(in crate::core::bundle) fn reject_unsupported_bundle_symlink_entry(
    entry_name: &str,
    is_symlink: bool,
) -> AppResult<()> {
    if is_symlink {
        return Err(AppError::Validation(format!(
            "bundle archive entry uses unsupported symlink metadata: {entry_name}"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::reject_unsupported_bundle_symlink_entry;

    #[test]
    fn reject_unsupported_bundle_symlink_entry_reports_entry_path() {
        let error = reject_unsupported_bundle_symlink_entry("addons/WeakAuras/WeakAuras.lua", true)
            .expect_err("symlink bundle entry should fail");

        let message = error.to_string();
        assert!(message.contains("bundle archive entry"));
        assert!(message.contains("unsupported symlink metadata"));
        assert!(message.contains("addons/WeakAuras/WeakAuras.lua"));
    }

    #[test]
    fn reject_unsupported_bundle_symlink_entry_allows_regular_entries() {
        reject_unsupported_bundle_symlink_entry("manifest.toml", false)
            .expect("regular bundle entry should pass");
    }
}
