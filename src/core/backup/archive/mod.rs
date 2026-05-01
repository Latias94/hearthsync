mod create;
mod metadata;
mod restore;

#[cfg(test)]
mod tests;

pub use create::create_backup;
pub use restore::restore_backup;

pub(super) use metadata::read_backup_metadata_from_path;
pub(super) use restore::restore_backup_task;

#[cfg(test)]
pub(super) use create::reject_unsupported_backup_source_symlink;
#[cfg(test)]
pub(super) use restore::set_restore_test_failure_after;
