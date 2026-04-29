use super::model::{CurseForgeFile, CurseForgeFileDependency};
use crate::core::error::{AppError, AppResult};

pub(crate) const CURSEFORGE_HASH_ALGO_SHA1: u8 = 1;
pub(crate) const CURSEFORGE_HASH_ALGO_MD5: u8 = 2;

const CURSEFORGE_RELEASE_STABLE: u8 = 1;
const CURSEFORGE_RELEASE_BETA: u8 = 2;
const CURSEFORGE_RELEASE_ALPHA: u8 = 3;
const CURSEFORGE_REQUIRED_DEPENDENCY_RELATION_TYPE: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CurseForgeFileReleaseType {
    Stable,
    Beta,
    Alpha,
}

impl CurseForgeFileReleaseType {
    pub(super) fn rank(self) -> u8 {
        match self {
            Self::Stable => CURSEFORGE_RELEASE_STABLE,
            Self::Beta => CURSEFORGE_RELEASE_BETA,
            Self::Alpha => CURSEFORGE_RELEASE_ALPHA,
        }
    }
}

pub(super) fn validate_curseforge_release_type(file: &CurseForgeFile) -> AppResult<()> {
    if curseforge_file_release_rank(file.release_type).is_none() {
        return Err(AppError::Validation(format!(
            "CurseForge file `{}` release type must be one of 1 (stable), 2 (beta), or 3 (alpha)",
            file.id
        )));
    }

    Ok(())
}

pub(super) fn validate_curseforge_file_dependencies(file: &CurseForgeFile) -> AppResult<()> {
    for dependency in &file.dependencies {
        if dependency.mod_id == 0 {
            return Err(AppError::Validation(format!(
                "CurseForge file `{}` dependency mod id must be greater than zero",
                file.id
            )));
        }
        if dependency.relation_type == 0 {
            return Err(AppError::Validation(format!(
                "CurseForge file `{}` dependency relation type must be greater than zero",
                file.id
            )));
        }
    }

    Ok(())
}

pub(super) fn file_matches_curseforge_release_type(
    file: &CurseForgeFile,
    max_release_type: CurseForgeFileReleaseType,
) -> bool {
    let Some(rank) = curseforge_file_release_rank(file.release_type) else {
        return false;
    };
    rank <= max_release_type.rank()
}

pub(super) fn curseforge_hash_contract(algo: u8) -> Option<(usize, &'static str)> {
    match algo {
        CURSEFORGE_HASH_ALGO_SHA1 => Some((40, "SHA-1")),
        CURSEFORGE_HASH_ALGO_MD5 => Some((32, "MD5")),
        _ => None,
    }
}

pub(in crate::core::addon::provider) fn required_dependency_mod_ids_for_curseforge_file(
    source_mod_id: u32,
    dependencies: &[CurseForgeFileDependency],
) -> Vec<u32> {
    let mut dependency_mod_ids = dependencies
        .iter()
        .filter(|dependency| {
            dependency.relation_type == CURSEFORGE_REQUIRED_DEPENDENCY_RELATION_TYPE
        })
        .map(|dependency| dependency.mod_id)
        .filter(|mod_id| *mod_id != 0 && *mod_id != source_mod_id)
        .collect::<Vec<_>>();
    dependency_mod_ids.sort_unstable();
    dependency_mod_ids.dedup();
    dependency_mod_ids
}

fn curseforge_file_release_rank(release_type: u8) -> Option<u8> {
    match release_type {
        CURSEFORGE_RELEASE_STABLE => Some(CurseForgeFileReleaseType::Stable.rank()),
        CURSEFORGE_RELEASE_BETA => Some(CurseForgeFileReleaseType::Beta.rank()),
        CURSEFORGE_RELEASE_ALPHA => Some(CurseForgeFileReleaseType::Alpha.rank()),
        _ => None,
    }
}
