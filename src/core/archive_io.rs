use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::Path;

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::core::error::AppResult;

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
    zip.start_file(archive_path, options)?;
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
