use std::path::{Path, PathBuf};

pub(in crate::core) fn join_segments(root: &Path, segments: &[&str]) -> PathBuf {
    let mut path = root.to_path_buf();
    for segment in segments {
        path.push(segment);
    }
    path
}

pub(in crate::core) fn to_zip_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
