use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::core::error::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize)]
pub(super) struct AddonIndexGovernance {
    pub schema_version: u32,
    pub name: String,
    pub updated_at: String,
    #[serde(default)]
    pub entries: Vec<AddonIndexGovernanceEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct AddonIndexGovernanceEntry {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub upstream_hosts: Vec<String>,
    pub source_attribution: String,
    #[serde(default)]
    pub maintainer: Option<String>,
    pub status: String,
    pub confidence: String,
    pub last_verified_at: String,
    #[serde(default)]
    pub notes: Option<String>,
}

impl AddonIndexGovernance {
    pub(super) fn searchable_terms_for_package(&self, package_id: &str) -> Vec<String> {
        self.entries
            .iter()
            .find(|entry| entry.id.eq_ignore_ascii_case(package_id))
            .map(AddonIndexGovernanceEntry::searchable_terms)
            .unwrap_or_default()
    }
}

impl AddonIndexGovernanceEntry {
    fn searchable_terms(&self) -> Vec<String> {
        let mut terms = self.aliases.clone();
        terms.extend(self.upstream_hosts.iter().cloned());
        terms.push(self.source_attribution.clone());
        if let Some(maintainer) = self.maintainer.as_deref() {
            terms.push(maintainer.to_string());
        }
        if let Some(notes) = self.notes.as_deref() {
            terms.push(notes.to_string());
        }

        terms
    }
}

pub(super) fn load_addon_index_governance(
    index_path: &Path,
) -> AppResult<Option<AddonIndexGovernance>> {
    let governance_path = addon_index_governance_path(index_path);
    if !governance_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&governance_path)?;
    let governance = serde_json::from_str::<AddonIndexGovernance>(&content)?;
    validate_addon_index_governance(&governance)?;
    Ok(Some(governance))
}

fn addon_index_governance_path(index_path: &Path) -> PathBuf {
    let mut path = index_path.to_path_buf();
    path.set_extension("governance.json");
    path
}

fn validate_addon_index_governance(governance: &AddonIndexGovernance) -> AppResult<()> {
    if governance.schema_version != 1 {
        return Err(AppError::Validation(format!(
            "unsupported addon index governance schema version: {}",
            governance.schema_version
        )));
    }
    if governance.name.trim().is_empty() {
        return Err(AppError::Validation(
            "addon index governance name must not be empty".to_string(),
        ));
    }
    if governance.updated_at.trim().is_empty() {
        return Err(AppError::Validation(
            "addon index governance updated_at must not be empty".to_string(),
        ));
    }
    if governance.entries.is_empty() {
        return Err(AppError::Validation(
            "addon index governance must contain at least one entry".to_string(),
        ));
    }

    let mut ids = BTreeSet::new();
    for entry in &governance.entries {
        validate_governance_entry(entry)?;
        let normalized_id = entry.id.trim().to_ascii_lowercase();
        if !ids.insert(normalized_id) {
            return Err(AppError::Validation(format!(
                "addon index governance contains duplicate package id: {}",
                entry.id
            )));
        }
    }

    Ok(())
}

fn validate_governance_entry(entry: &AddonIndexGovernanceEntry) -> AppResult<()> {
    validate_non_empty_text(
        &entry.id,
        "addon index governance package id",
        "addon index governance package id must not be empty",
    )?;
    if entry.aliases.is_empty() {
        return Err(AppError::Validation(format!(
            "addon index governance package `{}` must contain at least one alias",
            entry.id
        )));
    }
    validate_string_list(
        &entry.aliases,
        "addon index governance aliases",
        "addon index governance alias",
    )?;
    if entry.upstream_hosts.is_empty() {
        return Err(AppError::Validation(format!(
            "addon index governance package `{}` must contain at least one upstream host",
            entry.id
        )));
    }
    validate_upstream_hosts(&entry.upstream_hosts, &entry.id)?;
    validate_non_empty_text(
        &entry.source_attribution,
        "addon index governance source attribution",
        &format!(
            "addon index governance package `{}` source attribution must not be empty",
            entry.id
        ),
    )?;
    if let Some(maintainer) = entry.maintainer.as_deref() {
        validate_non_empty_text(
            maintainer,
            "addon index governance maintainer",
            &format!(
                "addon index governance package `{}` maintainer must not be empty",
                entry.id
            ),
        )?;
    }
    match entry.status.as_str() {
        "active" | "legacy" | "archived" | "blocked" => {}
        other => {
            return Err(AppError::Validation(format!(
                "addon index governance package `{}` has unsupported status `{other}`",
                entry.id
            )));
        }
    }
    match entry.confidence.as_str() {
        "high" | "medium" | "low" => {}
        other => {
            return Err(AppError::Validation(format!(
                "addon index governance package `{}` has unsupported confidence `{other}`",
                entry.id
            )));
        }
    }
    validate_non_empty_text(
        &entry.last_verified_at,
        "addon index governance last_verified_at",
        &format!(
            "addon index governance package `{}` last_verified_at must not be empty",
            entry.id
        ),
    )?;
    if let Some(notes) = entry.notes.as_deref() {
        validate_non_empty_text(
            notes,
            "addon index governance notes",
            &format!(
                "addon index governance package `{}` notes must not be empty",
                entry.id
            ),
        )?;
    }

    Ok(())
}

fn validate_upstream_hosts(hosts: &[String], package_id: &str) -> AppResult<()> {
    validate_string_list(
        hosts,
        "addon index governance upstream hosts",
        "upstream host",
    )?;
    let mut normalized_hosts = BTreeSet::new();
    for host in hosts {
        if !host.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        }) {
            return Err(AppError::Validation(format!(
                "addon index governance package `{package_id}` upstream host `{host}` must contain only ASCII lowercase letters, digits, `-`, or `_`"
            )));
        }
        if !normalized_hosts.insert(host.trim().to_ascii_lowercase()) {
            return Err(AppError::Validation(format!(
                "addon index governance package `{package_id}` contains duplicate upstream host `{host}`"
            )));
        }
    }

    Ok(())
}

fn validate_string_list(values: &[String], field: &str, item_field: &str) -> AppResult<()> {
    let mut normalized_values = BTreeSet::new();
    for value in values {
        validate_non_empty_text(
            value,
            item_field,
            &format!("{field} entries must not be empty"),
        )?;
        let normalized = value.trim().to_ascii_lowercase();
        if !normalized_values.insert(normalized) {
            return Err(AppError::Validation(format!(
                "{field} contains duplicate value `{value}`"
            )));
        }
    }

    Ok(())
}

fn validate_non_empty_text(value: &str, field: &str, message: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("invalid {field}: {message}")));
    }

    Ok(())
}
