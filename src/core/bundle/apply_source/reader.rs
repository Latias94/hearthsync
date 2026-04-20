use std::fs::File;

use zip::ZipArchive;

#[derive(Debug)]
pub(in crate::core::bundle) enum ApplySourceReader {
    BundleArchive(ZipArchive<File>),
    ExternalPackageArchive(ZipArchive<File>),
    ExternalPackageDirectory,
}
