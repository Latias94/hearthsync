use std::fs;

use tempfile::tempdir;

use super::support::*;
use crate::core::bundle::*;
use crate::core::install::{HostPlatform, WowFlavor};
use crate::core::manifest::{ApplyDefaults, CharacterMappingMode, ResourceApplyPolicy};
use crate::core::task::{NeverCancel, TaskKind, TaskPhase, VecTaskProgressSink};

mod analysis;
mod apply;
mod bundle;
mod progress;
mod serialization;
mod validation;
