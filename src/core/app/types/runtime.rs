use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperStrategyValue {
    NativeRust,
}

impl Default for HelperStrategyValue {
    fn default() -> Self {
        Self::NativeRust
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalHelperPolicyValue {
    NativeOnly,
    PreferExternal,
}

impl Default for ExternalHelperPolicyValue {
    fn default() -> Self {
        Self::NativeOnly
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalHelperAvailabilityValue {
    NotRequested,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalHelperCapabilitiesValue {
    pub policy: ExternalHelperPolicyValue,
    pub availability: ExternalHelperAvailabilityValue,
    pub active_strategy: HelperStrategyValue,
}
