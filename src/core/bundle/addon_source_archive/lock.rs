use std::path::Path;

use crate::core::addon::lock::{AddonLock, read_addon_lock};
use crate::core::error::AppResult;

pub(in crate::core::bundle) fn read_generated_addon_lock(path: &Path) -> AppResult<AddonLock> {
    read_addon_lock(path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::read_generated_addon_lock;

    #[test]
    fn read_generated_addon_lock_reuses_addon_lock_validation() {
        let temp = tempdir().expect("temp dir");
        let lock_path = temp.path().join("lock.toml");
        fs::write(
            &lock_path,
            r#"
schema_version = 0
generated_at = "2026-04-29T00:00:00Z"
packages = []
"#,
        )
        .expect("lock file");

        let error =
            read_generated_addon_lock(&lock_path).expect_err("invalid lock should fail closed");

        assert!(
            error
                .to_string()
                .contains("unsupported addon lock schema version")
        );
    }
}
