use super::analysis::build_analysis;
use super::classify::classify_source_entries;
use super::source::{collect_source_entries, detect_source_kind};
use super::types::{AnalyzeExternalPackageRequest, ExternalPackageAnalysis};
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
    let source_entries = collect_source_entries(&source_path, source_kind)?;
    let (entries, warnings) = classify_source_entries(&source_entries)?;

    Ok(build_analysis(
        source_path,
        source_kind,
        source_entries.len(),
        entries,
        warnings,
    ))
}
