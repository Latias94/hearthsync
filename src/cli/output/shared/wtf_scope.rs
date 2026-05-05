use crate::core::app::{
    ConfigWtfScopeSummaryResult, ExternalPackageWtfScopeSummaryResult, WtfScopeRiskValue,
    WtfScopeValue,
};

pub(in crate::cli::output) fn format_external_package_wtf_scopes(
    scopes: &[ExternalPackageWtfScopeSummaryResult],
) -> String {
    format_wtf_scopes(scopes.iter().map(|scope| WtfScopeDisplay {
        scope: scope.scope,
        risk: scope.risk,
        count: scope.count,
    }))
}

pub(in crate::cli::output) fn format_config_wtf_scopes(
    scopes: &[ConfigWtfScopeSummaryResult],
) -> String {
    format_wtf_scopes(scopes.iter().map(|scope| WtfScopeDisplay {
        scope: scope.scope,
        risk: scope.risk,
        count: scope.count,
    }))
}

struct WtfScopeDisplay {
    scope: WtfScopeValue,
    risk: WtfScopeRiskValue,
    count: usize,
}

fn format_wtf_scopes(scopes: impl IntoIterator<Item = WtfScopeDisplay>) -> String {
    let entries = scopes
        .into_iter()
        .map(|scope| {
            format!(
                "{}({})={}",
                format_wtf_scope(scope.scope),
                format_wtf_scope_risk(scope.risk),
                scope.count
            )
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        "none".to_string()
    } else {
        entries.join(", ")
    }
}

fn format_wtf_scope(scope: WtfScopeValue) -> &'static str {
    match scope {
        WtfScopeValue::GlobalConfig => "global_config",
        WtfScopeValue::RootSavedVariables => "root_saved_variables",
        WtfScopeValue::AccountRootFile => "account_root_file",
        WtfScopeValue::AccountSavedVariables => "account_saved_variables",
        WtfScopeValue::CharacterSavedVariables => "character_saved_variables",
        WtfScopeValue::CharacterState => "character_state",
        WtfScopeValue::CacheLike => "cache_like",
        WtfScopeValue::Unknown => "unknown",
    }
}

fn format_wtf_scope_risk(risk: WtfScopeRiskValue) -> &'static str {
    match risk {
        WtfScopeRiskValue::Low => "low",
        WtfScopeRiskValue::Medium => "medium",
        WtfScopeRiskValue::High => "high",
        WtfScopeRiskValue::Unknown => "unknown",
    }
}
