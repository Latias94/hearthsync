use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::addon::policy::{
    AddonPolicyPin as DomainAddonPolicyPin, AddonReleaseChannel as DomainAddonReleaseChannel,
};
use crate::core::addon::{
    AddonDependencyResolutionCapability as DomainAddonDependencyResolutionCapability,
    AddonDependencyResolutionStrategy as DomainAddonDependencyResolutionStrategy,
    AddonPackageMetadata as DomainAddonPackageMetadata,
    AddonProviderOptions as DomainAddonProviderOptions,
    AddonProviderRetryPolicy as DomainAddonProviderRetryPolicy,
    AddonProviderSourceCapability as DomainAddonProviderSourceCapability,
    AddonSourceFamily as DomainAddonSourceFamily, AddonStatePaths as DomainAddonStatePaths,
    AddonStateStorageKind as DomainAddonStateStorageKind,
    HttpNoValidatorCachePolicy as DomainHttpNoValidatorCachePolicy,
};
use crate::core::error::{AppError, AppResult};

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

impl AddonPackageMetadataValue {
    pub(crate) fn from_domain(value: DomainAddonPackageMetadata) -> Self {
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

    pub(crate) fn into_domain(self) -> AppResult<DomainAddonPackageMetadata> {
        let Self {
            index_name,
            index_package_id,
            package_name,
            version,
            source_url,
            website_url,
            source_sha256,
            supported_flavors,
        } = self;

        validate_optional_metadata_text("index_name", index_name.as_deref())?;
        validate_optional_metadata_text("index_package_id", index_package_id.as_deref())?;
        validate_optional_metadata_text("package_name", package_name.as_deref())?;
        validate_optional_metadata_text("version", version.as_deref())?;
        validate_optional_metadata_text("source_url", source_url.as_deref())?;
        validate_optional_metadata_text("website_url", website_url.as_deref())?;
        validate_optional_metadata_text("source_sha256", source_sha256.as_deref())?;
        for flavor in &supported_flavors {
            if flavor.trim().is_empty() {
                return Err(AppError::Validation(
                    "addon package metadata supported_flavors must not contain empty values"
                        .to_string(),
                ));
            }
        }

        Ok(DomainAddonPackageMetadata {
            index_name,
            index_package_id,
            package_name,
            version,
            source_url,
            website_url,
            source_sha256,
            supported_flavors,
        })
    }
}

fn validate_optional_metadata_text(field: &str, value: Option<&str>) -> AppResult<()> {
    if value.is_some_and(|value| value.trim().is_empty()) {
        return Err(AppError::Validation(format!(
            "addon package metadata {field} must not be empty"
        )));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonReleaseChannelValue {
    Stable,
    Beta,
    Alpha,
}

impl AddonReleaseChannelValue {
    pub(crate) fn from_domain(value: DomainAddonReleaseChannel) -> Self {
        match value {
            DomainAddonReleaseChannel::Stable => Self::Stable,
            DomainAddonReleaseChannel::Beta => Self::Beta,
            DomainAddonReleaseChannel::Alpha => Self::Alpha,
        }
    }

    pub(crate) fn into_domain(self) -> DomainAddonReleaseChannel {
        match self {
            Self::Stable => DomainAddonReleaseChannel::Stable,
            Self::Beta => DomainAddonReleaseChannel::Beta,
            Self::Alpha => DomainAddonReleaseChannel::Alpha,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddonPolicyPinValue {
    Version { value: String },
    FileId { value: u32 },
}

impl AddonPolicyPinValue {
    pub(crate) fn from_domain(value: DomainAddonPolicyPin) -> Self {
        match value {
            DomainAddonPolicyPin::Version { value } => Self::Version { value },
            DomainAddonPolicyPin::FileId { value } => Self::FileId { value },
        }
    }

    pub(crate) fn into_domain(self) -> DomainAddonPolicyPin {
        match self {
            Self::Version { value } => DomainAddonPolicyPin::Version { value },
            Self::FileId { value } => DomainAddonPolicyPin::FileId { value },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonDependencyResolutionStrategyValue {
    MissingRequiredOnly,
}

impl AddonDependencyResolutionStrategyValue {
    pub(crate) fn from_domain(value: DomainAddonDependencyResolutionStrategy) -> Self {
        match value {
            DomainAddonDependencyResolutionStrategy::MissingRequiredOnly => {
                Self::MissingRequiredOnly
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AddonDependencyResolutionCapabilityValue {
    Unsupported,
    Supported {
        strategy: AddonDependencyResolutionStrategyValue,
    },
}

impl AddonDependencyResolutionCapabilityValue {
    pub(crate) fn from_domain(value: DomainAddonDependencyResolutionCapability) -> Self {
        match value {
            DomainAddonDependencyResolutionCapability::Unsupported => Self::Unsupported,
            DomainAddonDependencyResolutionCapability::Supported { strategy } => Self::Supported {
                strategy: AddonDependencyResolutionStrategyValue::from_domain(strategy),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AddonProviderModeValue {
    ConfiguredDefault { options: AddonProviderOptionsValue },
    InternalCustom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonStateStorageValue {
    AppData,
    Sidecar,
}

impl AddonStateStorageValue {
    pub(crate) fn from_domain(value: DomainAddonStateStorageKind) -> Self {
        match value {
            DomainAddonStateStorageKind::AppData => Self::AppData,
            DomainAddonStateStorageKind::Sidecar => Self::Sidecar,
        }
    }

    pub(crate) fn into_domain(self) -> DomainAddonStateStorageKind {
        match self {
            Self::AppData => DomainAddonStateStorageKind::AppData,
            Self::Sidecar => DomainAddonStateStorageKind::Sidecar,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonStatePathsValue {
    pub root_dir: PathBuf,
    pub registry_path: PathBuf,
    pub lock_path: PathBuf,
    pub policy_path: PathBuf,
    pub adopted_dir: PathBuf,
}

impl AddonStatePathsValue {
    pub(crate) fn from_domain(value: DomainAddonStatePaths) -> Self {
        Self {
            root_dir: value.root_dir,
            registry_path: value.registry_path,
            lock_path: value.lock_path,
            policy_path: value.policy_path,
            adopted_dir: value.adopted_dir,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonManagementCapabilitiesValue {
    pub state_storage: AddonStateStorageValue,
    pub scan_only_without_managed_state: bool,
    pub managed_mode_requires_state: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddonSourceFamilyValue {
    LocalArchive,
    HttpArchive,
    CurseForgeMod,
    GitHubRelease,
}

impl AddonSourceFamilyValue {
    pub(crate) fn from_domain(value: DomainAddonSourceFamily) -> Self {
        match value {
            DomainAddonSourceFamily::LocalArchive => Self::LocalArchive,
            DomainAddonSourceFamily::HttpArchive => Self::HttpArchive,
            DomainAddonSourceFamily::CurseForgeMod => Self::CurseForgeMod,
            DomainAddonSourceFamily::GitHubRelease => Self::GitHubRelease,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonProviderSourceCapabilityValue {
    pub source_family: AddonSourceFamilyValue,
    pub provider_id: String,
    pub provider_name: String,
    pub input_prefix: Option<String>,
    pub can_parse_input: bool,
    pub can_materialize: bool,
    pub can_search: bool,
    pub dependency_resolution: AddonDependencyResolutionCapabilityValue,
    pub supports_release_channel: bool,
    pub supports_prerelease: bool,
    pub supports_version_pin: bool,
    pub supports_file_id_pin: bool,
    pub supports_remote_cache_validators: bool,
}

impl AddonProviderSourceCapabilityValue {
    pub(crate) fn from_domain(value: DomainAddonProviderSourceCapability) -> Self {
        Self {
            source_family: AddonSourceFamilyValue::from_domain(value.source_family),
            provider_id: value.provider_id.to_string(),
            provider_name: value.provider_name.to_string(),
            input_prefix: value.input_prefix.map(str::to_string),
            can_parse_input: value.can_parse_input,
            can_materialize: value.can_materialize,
            can_search: value.can_search,
            dependency_resolution: AddonDependencyResolutionCapabilityValue::from_domain(
                value.dependency_resolution,
            ),
            supports_release_channel: value.supports_release_channel,
            supports_prerelease: value.supports_prerelease,
            supports_version_pin: value.supports_version_pin,
            supports_file_id_pin: value.supports_file_id_pin,
            supports_remote_cache_validators: value.supports_remote_cache_validators,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppRuntimeCapabilitiesValue {
    pub addon_provider: AddonProviderModeValue,
    pub addon_source_capabilities: Vec<AddonProviderSourceCapabilityValue>,
    pub addon_management: AddonManagementCapabilitiesValue,
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

impl AddonProviderRetryPolicyValue {
    #[cfg(test)]
    pub(crate) fn from_domain(value: DomainAddonProviderRetryPolicy) -> Self {
        Self {
            max_attempts: value.max_attempts,
        }
    }

    pub(crate) fn into_domain(self) -> AppResult<DomainAddonProviderRetryPolicy> {
        if self.max_attempts == 0 {
            return Err(AppError::Validation(
                "addon provider retry policy max_attempts must be greater than zero".to_string(),
            ));
        }

        Ok(DomainAddonProviderRetryPolicy {
            max_attempts: self.max_attempts,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum HttpNoValidatorCachePolicyValue {
    AlwaysRefresh,
    ReuseWithinWindow { max_age_secs: u64 },
}

impl Default for HttpNoValidatorCachePolicyValue {
    fn default() -> Self {
        Self::ReuseWithinWindow { max_age_secs: 900 }
    }
}

impl HttpNoValidatorCachePolicyValue {
    #[cfg(test)]
    pub(crate) fn from_domain(value: DomainHttpNoValidatorCachePolicy) -> Self {
        match value {
            DomainHttpNoValidatorCachePolicy::AlwaysRefresh => Self::AlwaysRefresh,
            DomainHttpNoValidatorCachePolicy::ReuseWithinWindow { max_age_secs } => {
                Self::ReuseWithinWindow { max_age_secs }
            }
        }
    }

    pub(crate) fn into_domain(self) -> AppResult<DomainHttpNoValidatorCachePolicy> {
        match self {
            Self::AlwaysRefresh => Ok(DomainHttpNoValidatorCachePolicy::AlwaysRefresh),
            Self::ReuseWithinWindow { max_age_secs } => {
                if max_age_secs == 0 {
                    return Err(AppError::Validation(
                        "HTTP no-validator cache window must be greater than zero seconds"
                            .to_string(),
                    ));
                }

                Ok(DomainHttpNoValidatorCachePolicy::ReuseWithinWindow { max_age_secs })
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddonProviderOptionsValue {
    #[serde(default)]
    pub download_cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub retry_policy: AddonProviderRetryPolicyValue,
    #[serde(default)]
    pub http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue,
}

impl AddonProviderOptionsValue {
    #[cfg(test)]
    pub(crate) fn from_domain(value: DomainAddonProviderOptions) -> Self {
        Self {
            download_cache_dir: value.download_cache_dir,
            retry_policy: AddonProviderRetryPolicyValue::from_domain(value.retry_policy),
            http_no_validator_cache_policy: HttpNoValidatorCachePolicyValue::from_domain(
                value.http_no_validator_cache_policy,
            ),
        }
    }

    pub(crate) fn into_domain(self) -> AppResult<DomainAddonProviderOptions> {
        Ok(DomainAddonProviderOptions {
            download_cache_dir: self.download_cache_dir,
            retry_policy: self.retry_policy.into_domain()?,
            http_no_validator_cache_policy: self.http_no_validator_cache_policy.into_domain()?,
        })
    }
}
