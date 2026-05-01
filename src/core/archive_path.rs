mod platform;
mod segment;
mod zip;

#[cfg(test)]
mod tests;

pub(in crate::core) use platform::{
    PlatformPathCollisionKind, PlatformPathPrefixConflictKind, find_platform_path_collision,
    find_platform_path_prefix_conflict, platform_path_collision_key,
};
pub(in crate::core) use segment::{
    safe_relative_segments, safe_zip_segments, safe_zip_segments_under,
    validate_portable_path_segment,
};
pub(in crate::core) use zip::{join_segments, to_zip_path};
