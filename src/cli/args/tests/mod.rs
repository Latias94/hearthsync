use std::path::PathBuf;

use clap::Parser;

use super::addon::{AddonCacheCommands, AddonIndexCommands, AddonPolicyCommands};
use super::config::ConfigCommands;
use super::settings::SettingsCommands;
use super::shared::{
    AddonCacheRepairRemotePolicyArg, AddonStateStorageArg, FlavorArg, ReleaseChannelArg,
};
use super::{AddonCommands, Cli, Commands};

mod addon;
mod addon_index;
mod config;
mod runtime;
mod settings;
