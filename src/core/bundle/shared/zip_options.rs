use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

pub(in crate::core::bundle) fn zip_file_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Deflated)
}

pub(in crate::core::bundle) fn zip_dir_options() -> SimpleFileOptions {
    SimpleFileOptions::default().compression_method(CompressionMethod::Stored)
}
