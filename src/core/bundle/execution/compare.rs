use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use super::super::*;

pub(in crate::core::bundle) fn file_contents_equal_to_bytes(
    bytes: &[u8],
    right: &Path,
) -> AppResult<bool> {
    if !right.exists() || !right.is_file() {
        return Ok(false);
    }

    let right_metadata = fs::metadata(right)?;
    if right_metadata.len() != bytes.len() as u64 {
        return Ok(false);
    }

    let mut right_file = File::open(right)?;
    let mut right_buffer = [0u8; 8192];
    let mut offset = 0usize;

    loop {
        let right_read = right_file.read(&mut right_buffer)?;
        if right_read == 0 {
            return Ok(offset == bytes.len());
        }
        if offset + right_read > bytes.len() {
            return Ok(false);
        }
        if bytes[offset..offset + right_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        offset += right_read;
    }
}
