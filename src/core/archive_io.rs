use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::Path;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::core::archive_path::safe_zip_segments;
use crate::core::error::AppResult;

pub(crate) fn start_file_to_zip<W>(
    zip: &mut ZipWriter<W>,
    archive_path: &str,
    options: SimpleFileOptions,
) -> AppResult<()>
where
    W: Write + Seek,
{
    validate_zip_archive_path(archive_path)?;
    zip.start_file(archive_path, options)?;
    Ok(())
}

pub(crate) fn add_directory_to_zip<W>(
    zip: &mut ZipWriter<W>,
    archive_path: &str,
    options: SimpleFileOptions,
) -> AppResult<()>
where
    W: Write + Seek,
{
    validate_zip_archive_path(archive_path)?;
    zip.add_directory(archive_path, options)?;
    Ok(())
}

pub(crate) fn stream_file_to_zip<W>(
    zip: &mut ZipWriter<W>,
    source_path: &Path,
    archive_path: &str,
    options: SimpleFileOptions,
) -> AppResult<()>
where
    W: Write + Seek,
{
    let mut file = File::open(source_path)?;
    start_file_to_zip(zip, archive_path, options)?;
    std::io::copy(&mut file, zip)?;
    Ok(())
}

pub(crate) fn copy_reader_to_path<R>(reader: &mut R, destination: &Path) -> AppResult<()>
where
    R: Read,
{
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = File::create(destination)?;
    std::io::copy(reader, &mut output)?;
    Ok(())
}

fn validate_zip_archive_path(archive_path: &str) -> AppResult<()> {
    safe_zip_segments(archive_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    use super::{add_directory_to_zip, start_file_to_zip};

    #[test]
    fn start_file_to_zip_rejects_non_portable_archive_paths() {
        let cursor = Cursor::new(Vec::<u8>::new());
        let mut zip = ZipWriter::new(cursor);

        let error = start_file_to_zip(
            &mut zip,
            "addons/CON/Config.lua",
            SimpleFileOptions::default(),
        )
        .expect_err("non-portable archive path should fail");

        assert!(error.to_string().contains("unsafe archive path"));
    }

    #[test]
    fn add_directory_to_zip_rejects_non_portable_archive_paths() {
        let cursor = Cursor::new(Vec::<u8>::new());
        let mut zip = ZipWriter::new(cursor);

        let error = add_directory_to_zip(&mut zip, "addons/CON", SimpleFileOptions::default())
            .expect_err("non-portable archive directory should fail");

        assert!(error.to_string().contains("unsafe archive path"));
    }
}
