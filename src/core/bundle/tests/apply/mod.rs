use std::cell::Cell;
use std::fs;

use tempfile::tempdir;
use zip::ZipArchive;

use super::super::constants::MANIFEST_ENTRY;
use super::super::planner::pipeline::{plan_apply_from_entries_with_reader, prepare_bundle_apply};
use super::support::*;
use crate::core::bundle::*;
use crate::core::install::HostPlatform;
use crate::core::manifest::{CharacterMappingMode, CharacterResource, ResourceApplyPolicy};
use crate::core::task::{CancellationToken, NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

mod execution;
mod planning;
mod policy;
mod progress;
