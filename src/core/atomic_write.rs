use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;

use crate::core::error::AppResult;

pub(in crate::core) fn write_bytes_atomically(path: &Path, contents: &[u8]) -> AppResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(contents)?;
    temporary.flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;

    sync_parent_dir(parent)?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn sync_parent_dir(path: &Path) -> AppResult<()> {
    std::fs::File::open(path)?.sync_all()?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn sync_parent_dir(_path: &Path) -> AppResult<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::write_bytes_atomically;

    #[test]
    fn write_bytes_atomically_creates_new_file_without_leaving_temp_files() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("state.toml");

        write_bytes_atomically(&path, b"schema_version = 1\n").expect("atomic write");

        assert_eq!(
            fs::read_to_string(&path).expect("state file"),
            "schema_version = 1\n"
        );
        let entries = fs::read_dir(temp.path())
            .expect("temp dir entries")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(entries, vec!["state.toml".to_string()]);
    }

    #[test]
    fn write_bytes_atomically_replaces_existing_file_contents() {
        let temp = tempdir().expect("temp dir");
        let path = temp.path().join("state.toml");
        fs::write(&path, "old").expect("old file");

        write_bytes_atomically(&path, b"new").expect("replace atomic write");

        assert_eq!(fs::read_to_string(&path).expect("state file"), "new");
    }
}
