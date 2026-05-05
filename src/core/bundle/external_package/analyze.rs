use super::analysis::build_analysis;
use super::classify::classify_source_entries;
use super::source::{collect_source_entries, detect_source_kind};
use super::types::{AnalyzeExternalPackageRequest, ExternalPackageAnalysis, ExternalPackageLayout};
use crate::core::error::{AppError, AppResult};

pub fn analyze_external_package(
    request: AnalyzeExternalPackageRequest,
) -> AppResult<ExternalPackageAnalysis> {
    let source_path = request.source_path;
    if !source_path.exists() {
        return Err(AppError::NotFound(format!(
            "external package source does not exist: {}",
            source_path.display()
        )));
    }

    let source_kind = detect_source_kind(&source_path)?;
    let layout = resolve_external_package_layout(request.layout, &source_path);
    let source_entries = collect_source_entries(&source_path, source_kind, layout)?;
    let (entries, warnings) = classify_source_entries(
        &source_entries,
        layout,
        request.source_account.as_deref(),
        request.source_server.as_deref(),
        request.source_character.as_deref(),
    )?;

    Ok(build_analysis(
        source_path,
        source_kind,
        layout,
        source_entries.len(),
        entries,
        warnings,
    ))
}

fn resolve_external_package_layout(
    requested: ExternalPackageLayout,
    source_path: &std::path::Path,
) -> ExternalPackageLayout {
    if requested != ExternalPackageLayout::Auto {
        return requested;
    }

    let name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if name.starts_with("font-") {
        return ExternalPackageLayout::NewBeeBoxFont;
    }
    if name.starts_with("material-") {
        return ExternalPackageLayout::NewBeeBoxMaterial;
    }
    if name.starts_with("unknown_plug-") {
        return ExternalPackageLayout::NewBeeBoxAddon;
    }
    if is_newbeebox_module_cache_path(source_path) {
        return ExternalPackageLayout::NewBeeBoxAddon;
    }
    if name.starts_with("wtfserve-") {
        return ExternalPackageLayout::NewBeeBoxWtfAccount;
    }
    if name.starts_with("wtfrole-") {
        return ExternalPackageLayout::NewBeeBoxWtfCharacter;
    }

    ExternalPackageLayout::Generic
}

fn is_newbeebox_module_cache_path(source_path: &std::path::Path) -> bool {
    let Some(parent) = source_path.parent() else {
        return false;
    };
    let Some(parent_name) = parent.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    if !parent_name.eq_ignore_ascii_case("modules") {
        return false;
    }

    let Some(cache_name) = parent
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
    else {
        return false;
    };

    cache_name.eq_ignore_ascii_case("NewBeeBoxCache")
}
