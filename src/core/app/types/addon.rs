use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::addon::{
    AddonPackageMetadata as DomainAddonPackageMetadata,
    AddonProviderOptions as DomainAddonProviderOptions,
    AddonProviderRetryPolicy as DomainAddonProviderRetryPolicy,
};

use super::runtime::ExternalHelperCapabilitiesValue;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonPackageMetadataValue {
    #[serde(default)]
    pub index_name: Option<String>,
    #[serde(default)]
    pub index_package_id: Option<String>,
    #[serde(default)]
    pub package_name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub source_sha256: Option<String>,
    #[serde(default)]
    pub supported_flavors: Vec<String>,
}

impl From<DomainAddonPackageMetadata> for AddonPackageMetadataValue {
    fn from(value: DomainAddonPackageMetadata) -> Self {
        Self {
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            package_name: value.package_name,
            version: value.version,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            supported_flavors: value.supported_flavors,
        }
    }
}

impl From<AddonPackageMetadataValue> for DomainAddonPackageMetadata {
    fn from(value: AddonPackageMetadataValue) -> Self {
        Self {
            index_name: value.index_name,
            index_package_id: value.index_package_id,
            package_name: value.package_name,
            version: value.version,
            source_url: value.source_url,
            website_url: value.website_url,
            source_sha256: value.source_sha256,
            supported_flavors: value.supported_flavors,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AddonProviderModeValue {
    ConfiguredDefault { options: AddonProviderOptionsValue },
    InternalCustom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRuntimeCapabilitiesValue {
    pub addon_provider: AddonProviderModeValue,
    pub external_helper: ExternalHelperCapabilitiesValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonProviderRetryPolicyValue {
    pub max_attempts: u32,
}

impl Default for AddonProviderRetryPolicyValue {
    fn default() -> Self {
        Self { max_attempts: 1 }
    }
}

impl From<DomainAddonProviderRetryPolicy> for AddonProviderRetryPolicyValue {
    fn from(value: DomainAddonProviderRetryPolicy) -> Self {
        Self {
            max_attempts: value.max_attempts,
        }
    }
}

impl From<AddonProviderRetryPolicyValue> for DomainAddonProviderRetryPolicy {
    fn from(value: AddonProviderRetryPolicyValue) -> Self {
        Self {
            max_attempts: value.max_attempts,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonProviderOptionsValue {
    pub download_cache_dir: Option<PathBuf>,
    pub retry_policy: AddonProviderRetryPolicyValue,
}

impl From<DomainAddonProviderOptions> for AddonProviderOptionsValue {
    fn from(value: DomainAddonProviderOptions) -> Self {
        Self {
            download_cache_dir: value.download_cache_dir,
            retry_policy: value.retry_policy.into(),
        }
    }
}

impl From<AddonProviderOptionsValue> for DomainAddonProviderOptions {
    fn from(value: AddonProviderOptionsValue) -> Self {
        Self {
            download_cache_dir: value.download_cache_dir,
            retry_policy: value.retry_policy.into(),
        }
    }
}
